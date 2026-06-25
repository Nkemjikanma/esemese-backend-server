use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use sqlx::PgPool;
use tokio::task;
use uuid::Uuid;
use crate::common::errors::{AppError, DerivativesGenerationError, UploadsError};
use crate::config::Config;
use crate::image_processing::process_image;

pub async fn process_derivative_for_photo(id: Uuid, db_pool: PgPool, s3: Client, app_cfg: Config) -> Result<(), AppError>{
    // Fetch the s3_key,status/ from DB
    let photo_record = sqlx::query!(r#"SELECT s3_key, title FROM photos WHERE id=$1"#, id).fetch_one(&db_pool).await.map_err(|_| {
        tracing::error!("Error fetching image in generate derivative function");

        UploadsError::PhotoQueryError("Error fetching photo".to_string())
    })?;

    // download the image from s3
    let image_data = s3.get_object().bucket(&app_cfg.rustfs_config.bucket_photos).key(photo_record.s3_key).send().await.map_err(|_| {
        tracing::error!("Error downloading the original image from s3 bucket");

        DerivativesGenerationError::ObjectDownloadError
    })?;

    // Extract the bytes from the S3 Object output
    let image = image_data.body.collect().await.map_err(|_| {
        tracing::error!("Error extracting image content from downloaded object output");

        DerivativesGenerationError::BytesExtractionError
    })?;
    let image_bytes = image.into_bytes();

    // CPU-heavy processing - off to tokio runtime and rayon
    let processed = task::spawn_blocking(move || process_image(image_bytes.to_vec())).await.map_err(|e| {
        tracing::error!("Something went wrong during image processing");

        DerivativesGenerationError::ImageProcessingError(e.to_string())
    })??;


    // open the transaction BEFORE the loop so the variant inserts + status flip commit atomically
    let mut tx = db_pool.begin().await.map_err(|e| {
        tracing::error!("Failed to open transaction for variant persistence");
        DerivativesGenerationError::ErrorRecordingVariants(e.to_string())
    })?;

    // upload each derivative to S3, then record its variant row inside the transaction
    for d in &processed.derivatives {
        let key = format!("variants/{id}/{}.avif", d.hash);

        s3.put_object()
            .bucket(&app_cfg.rustfs_config.bucket_photos)
            .key(&key)
            .body(ByteStream::from(d.bytes.clone()))
            .content_type("image/avif")
            .send().await
            .map_err(|_| DerivativesGenerationError::ObjectUploadError)?;

        sqlx::query!(
            r#"INSERT INTO variants (photo_id, s3_key, width, height, format, byte_size)
               VALUES ($1, $2, $3, $4, 'avif', $5)
               ON CONFLICT (photo_id, width, format) DO NOTHING"#,
            id, key, d.width as i32, d.height as i32, d.bytes.len() as i64,
        ).execute(&mut *tx).await.map_err(|e| {
            tracing::error!("Failed to insert variant record");
            DerivativesGenerationError::ErrorRecordingVariants(e.to_string())
        })?;
    }

    // any photo reaching here has >= 1 derivative (process_image errors if all widths fail);
    sqlx::query!(
        r#"UPDATE photos SET blurhash=$1, status='ready' WHERE id=$2"#,
        processed.blurhash, id,
    ).execute(&mut *tx).await.map_err(|e| {
        tracing::error!("Failed to flip photo status to ready");
        DerivativesGenerationError::ErrorRecordingVariants(e.to_string())
    })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit variant transaction");
        DerivativesGenerationError::ErrorRecordingVariants(e.to_string())
    })?;

    Ok(())
}
