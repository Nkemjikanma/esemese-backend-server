use crate::common::errors::DerivativesGenerationError;
use crate::types::derivatives::PhotoMetadata;
use blurhash::encode;
use chrono::{DateTime, NaiveDate, Utc};
use exif;
use image::{imageops, DynamicImage, EncodableLayout, ExtendedColorType, ImageDecoder, ImageEncoder, ImageReader};
use rayon::iter::IntoParallelRefIterator;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::io::Cursor;

const WIDTHS: [u32; 5] = [400, 800, 1200, 1600, 2400];
pub struct Derivative {
	pub width: u32,
	pub height: u32,
	pub bytes: Vec<u8>,
	pub hash: String, // sha256 hex of bytes -> for the s_3 key
}

pub struct Processed {
	pub derivatives: Vec<Derivative>,
	pub failed_widths: Vec<u32>,
	pub blurhash: String,
	pub photo_metadata: PhotoMetadata,
}

pub fn process_image(image_bytes: Vec<u8>) -> Result<Processed, DerivativesGenerationError> {
	// get exif data
	let exif_reader = exif::Reader::new();
	// Absence of exif must not be fatal so it doesn't abort derivative generation No EXIF is perfectly valid too
	let exif = match exif_reader.read_from_container(&mut Cursor::new(&image_bytes)) {
		Ok(e) => Some(e),
		Err(_) => {
			tracing::error!("no EXIF for photo; proceeding with empty metadata");
			None
		}
	};

	let camera = exif.as_ref()
		.and_then(|e| e.get_field(exif::Tag::Model, exif::In::PRIMARY)
			.map(|f| f.display_value().to_string())); // watch for what value this returns
	let lens = exif.as_ref()
		.and_then(|e| e.get_field(exif::Tag::LensModel, exif::In::PRIMARY)
			.map(|f| f.display_value().to_string())); // asWell as this - they are
	let iso = exif.as_ref()
		.and_then(|e| e.get_field(exif::Tag::PhotographicSensitivity, exif::In::PRIMARY)
			.map(|f| f.value.get_uint(0)));
	let aperture = exif.as_ref()
		.and_then(|e| e.get_field(exif::Tag::FNumber, exif::In::PRIMARY)
			.map(|f| f.display_value().to_string()));
	let shutter_speed = exif.as_ref()
		.and_then(|e| e.get_field(exif::Tag::ExposureTime, exif::In::PRIMARY)
			.map(|f| f.display_value().to_string()));
	let focal_length = exif.as_ref()
		.and_then(|e| e.get_field(exif::Tag::FocalLength, exif::In::PRIMARY)
			.map(|f| f.display_value().to_string()));
	let taken_at: Option<DateTime<Utc>> = exif.as_ref()
		.and_then(|e| e.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
			.and_then(|f| match &f.value {
				exif::Value::Ascii(v) if !v.is_empty() => exif::DateTime::from_ascii(&v[0]).ok(),
				_ => None,
	}).and_then(|dt| chrono::NaiveDate::from_ymd_opt(dt.year as i32, dt.month.into(), dt.day.into())?
			.and_hms_opt(dt.hour.into(), dt.minute.into(), dt.second.into()))
			.map(|naive| naive.and_utc())); // date-time when shot
	// Load bytes into image crate for to prepare for converting
	let image_reader = ImageReader::new(Cursor::new(&image_bytes))
		.with_guessed_format().map_err(|e| {
		tracing::error!("There was a problem reading image bytes for processing: {:?}", e);
		DerivativesGenerationError::ImageByteReadingError
	})?;

	// get orientation
	let mut image_reader_decoder = image_reader.into_decoder()?;
	let orientation = image_reader_decoder.orientation()?;

	let mut image = DynamicImage::from_decoder(image_reader_decoder).map_err(|_| {
		tracing::error!("Failed to extract image from the reader");
		DerivativesGenerationError::ReaderExtractionError
	})?;

	// let image = image.apply_orientation(image_orientation);
	image.apply_orientation(orientation);

	// create the blurhash
	// let (width, height) = image.dimensions();
	let low_quality_image = image.resize(32, 32, imageops::FilterType::Lanczos3).to_rgba8();
	let (low_q_width, low_q_height) = low_quality_image.dimensions();
	let blur_hash = encode(4, 3, low_q_width, low_q_height, low_quality_image.as_bytes()).map_err(|_| {
		tracing::error!("Blurhash generation error");
		DerivativesGenerationError::BlurhashCreationError
	})?;

	// resize and convert in parrallel
	let results: Vec<(u32, Result<Derivative, _>)> = generate_derivative(&WIDTHS, &image);

	// both halves are Vecs of the same tuple types
	let (succeeded, failed): (Vec<_>, Vec<_>) = results.into_iter().partition(|(_, r)| r.is_ok());

	let derivatives: Vec<Derivative> = succeeded.into_iter().map(|(_, r)| r.unwrap()).collect();
	let failed_widths: Vec<u32> = failed.into_iter().map(|(w, _)| w).collect();


	// if all derivatives failed
	if derivatives.is_empty() {
		tracing::warn!("All {} witdths failed to encode...", WIDTHS.len());

		return Err(DerivativesGenerationError::AVIFEncodingError);
	}

	// for partial failures - not fatal - we log this failure
	if !failed_widths.is_empty() {
		tracing::warn!("{} of {} widths failed to encode", failed_widths.len(), WIDTHS.len());
	}

	// extract metadata
	let photo_metadata = PhotoMetadata {
		camera,
		lens,
		iso,
		aperture,
		shutter_speed,
		focal_length,
		taken_at
	};


	Ok(Processed { derivatives, failed_widths, blurhash: blur_hash, photo_metadata })
}


fn encode_width(image: &DynamicImage, width: u32) -> Result<Derivative, DerivativesGenerationError> {
	// resize per width in parallel, use u32::MAX to compute height proportionally
	// using Lanczos3 for best image quality but slower
	let resized = image.resize(width, u32::MAX, imageops::FilterType::Lanczos3);
	let height = resized.height();

	let mut out = Vec::new();

	let rgba = resized.to_rgba8();
	// speed 6–8 (1–10 scale, higher = faster) and quality ~50–70 for web derivatives.
	let new_image_quality = image::codecs::avif::AvifEncoder::new_with_speed_quality(&mut out, 8, 70);
	new_image_quality.write_image(rgba.as_raw(), resized.width(), resized.height(), ExtendedColorType::Rgba8).map_err(|_| {
		tracing::error!("Failed to save resized and encoded image");

		DerivativesGenerationError::AVIFEncodingError
	})?;

	let hash = Sha256::digest(&out).iter().map(|b| format!("{b:02x}")).collect::<String>();

	Ok(Derivative {
		width,
		height,
		bytes: out,
		hash,
	})
}

fn generate_derivative(widths: &[u32; 5], image: &DynamicImage) -> Vec<(u32, Result<Derivative, DerivativesGenerationError>)> {
	widths.par_iter().map(|&width| (width, encode_width(image, width))).collect()
}