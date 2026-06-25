use std::io::Cursor;
use blurhash::encode;
use image::{imageops, DynamicImage, EncodableLayout, GenericImageView, ImageFormat, ImageReader};
use rayon::iter::IntoParallelRefIterator;
use sha2::{Digest, Sha256};
use rayon::prelude::*;
use crate::common::errors::DerivativesGenerationError;

const WIDTHS: [u32; 5]= [400, 800, 1200, 1600, 2400];
pub struct Derivative {
   pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
    pub hash: String, // sha256 hex of bytes -> for the s_3 key
}

pub struct Processed {
   pub derivatives: Vec<Derivative>,
    pub failed_widths: Vec<u32>,
    pub blurhash: String
}

 pub fn process_image(image_bytes: Vec<u8>) -> Result<Processed, DerivativesGenerationError> {
     // Load bytes into image crate for to prepare for converting
     let image_reader = ImageReader::new(Cursor::new(image_bytes)).with_guessed_format().map_err(|_| {
         tracing::error!("There was a problem reading image bytes for processing");
         DerivativesGenerationError::ImageByteReadingError
     })?;

     let image: DynamicImage = image_reader.decode().map_err(|_| {
         tracing::error!("Failed to extract image from the reader");
         DerivativesGenerationError::ReaderExtractionError
     })?;

     // create the blurhash
     let (width, height) = image.dimensions();
     let low_quality_image = image.resize(32, 32, imageops::FilterType::Lanczos3).to_rgb8();
     let blur_hash = encode(4, 3, width, height, low_quality_image.as_bytes()).map_err(|_| {
         tracing::error!("Blurhash generation error");
         DerivativesGenerationError::BlurhashCreationError
     })?;

     // resize and convert in parrallel
     let results: Vec<(u32, Result<Derivative, _>)> = generate_derivative(&WIDTHS, &image);

     // both halves are Vecs of the same tuple types
     let (succeeded, failed): (Vec<_>, Vec<_>) = results.into_iter().partition(|(_, r)| r.is_ok());

     let derivatives: Vec<Derivative> = succeeded.into_iter().map(|(_, r)| r.unwrap()).collect();
     let failed_widths : Vec<u32>= failed.into_iter().map(|(w, _)| w).collect();


     // if all derivatives failed
     if derivatives.is_empty() {
         tracing::warn!("All {} witdths failed to encode...", WIDTHS.len());

         return Err(DerivativesGenerationError::AVIFEncodingError);
     }

     // for partial failures - not fatal - we log this failure
     if !failed_widths.is_empty() {
         tracing::warn!("{} of {} widths failed to encode", failed_widths.len(), WIDTHS.len());
     }

     Ok(Processed { derivatives, failed_widths, blurhash: blur_hash})
 }


fn encode_width(image: &DynamicImage, width:u32) -> Result<Derivative, DerivativesGenerationError> {
    // resize per width in parallel, use u32::MAX to compute height proportionally
    // using Lanczos3 for best image quality but slower
    let resized = image.resize(width, u32::MAX,  imageops::FilterType::Lanczos3);
    let height = resized.height();

    let mut out = Vec::new();

    resized.write_to(&mut Cursor::new(&mut out), ImageFormat::Avif).map_err(|_| {
        tracing::error!("Failed to save resized and encoded image");

        DerivativesGenerationError::AVIFEncodingError
    })?;

    let hash = Sha256::digest(&out).iter().map(|b| format!("{b:02x}")).collect::<String>();

    Ok(Derivative {
        width,
        height,
        bytes: out,
        hash
    })
}

fn generate_derivative(widths: &[u32; 5], image: &DynamicImage) -> Vec<(u32, Result<Derivative, DerivativesGenerationError>)> {
    widths.par_iter().map(|&width| (width, encode_width(image, width))).collect()
}