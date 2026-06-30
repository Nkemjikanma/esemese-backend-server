use crate::common::errors::{AppError, UploadsError};
use std::time::Duration;
use aws_sdk_s3::{presigning::PresigningConfig};
use crate::{types::{uploads::{InitiateUploadsRequest, ConfirmUploadsRequest, ConfirmUploadsResponse, InitiateUploadResponse, PhotoStatus}, app::AppState},};
use uuid::Uuid;


const MAX_PHOTO_SIZE_BYTES: i64 = 50 * 1024 * 1024;
const MIN_PHOTO_SIZE_BYTES: i64 = 2 * 1024 * 1024;

pub struct UploadsService;

impl UploadsService {
    #[tracing::instrument(name = "request_presigned_url", skip(app_state))]
    pub async fn request_presigned_url(request_body: &InitiateUploadsRequest, app_state: &AppState) -> Result<InitiateUploadResponse, AppError> {
        let AppState {rustfs_client,  app_config, ..} = app_state;

        let InitiateUploadsRequest {content_type, file_size, file_name} = request_body;

        // check image size
        if file_size < &MIN_PHOTO_SIZE_BYTES || file_size > &MAX_PHOTO_SIZE_BYTES {
            tracing::error!("File provided is outside the file limits");
            return Err(AppError::Uploads(UploadsError::InvalidFileSize(file_size.to_string())))
        }

        // generate the Uuid
        let photo_id = Uuid::new_v4();

        let ext = content_type.extension();
        let s3_key = format!("originals/{photo_id}.{ext}");

        tracing::info!("Adding new upload request to DB");
        sqlx::query!(
            r#"INSERT INTO photos (id, status, s3_key, original_filename, file_size, mime_type) VALUES ($1, 'initiated'::photo_status, $2, $3, $4, $5)"#
            , photo_id, s3_key, file_name, file_size, content_type.mime()).execute(&app_state.connection).await.map_err(|err| {
            tracing::error!("Error initializing upload - new row insert failed");
            UploadsError::UploadInitiationError(err.to_string())
        })?;

        // Presiging step
        let expires_in: Duration = Duration::from_secs(900);
        let expires_in = PresigningConfig::expires_in(expires_in).map_err(|err| {
            tracing::error!(%err, "failed to build presigning config");
            UploadsError::PresignedURIGenerationError(err.to_string())
        })?;

        let presigned_request = rustfs_client.put_object().bucket(&app_config.rustfs_config.bucket_photos).key(s3_key).content_type(content_type.mime()).presigned(expires_in).await.map_err(|err| {
            tracing::error!(%err, "failed to generate presigned request");
            UploadsError::PresignedURIGenerationError(err.to_string())
        })?;

        let response =  InitiateUploadResponse {
           photo_id,
            uri: presigned_request.uri().to_string()
        };

        // return photo_id and presigned uri
        Ok(response)
    }

    #[tracing::instrument(name = "confirming upload exists", skip(app_state))]
    pub async fn confirm_upload_exists(request_body: &ConfirmUploadsRequest, app_state: &AppState) -> Result<ConfirmUploadsResponse, AppError> {
        tracing::info!("Confirming the upload to bucket");
        let ConfirmUploadsRequest {photo_id, title, description, category} = request_body;
        let AppState {connection, rustfs_client, app_config, notify_on_confirm} = app_state;

        let photo_record = sqlx::query!(r#"SELECT id, s3_key, status AS "status: PhotoStatus" FROM photos WHERE id = $1"#, photo_id).fetch_one(connection).await.map_err(|err: sqlx::Error| {
           tracing::error!("Can't find photo in our DB");

            match err {
                sqlx::Error::RowNotFound => UploadsError::PhotoNotFound(photo_id.to_string()),
                _ => UploadsError::PhotoQueryError("Error fetching photo".to_string())
            }
        })?;

        if photo_record.status != PhotoStatus::Initiated {
            return Ok(ConfirmUploadsResponse {
                photo_id: *photo_id,
                status: photo_record.status,
            });
        }
        tracing::info!("Verying content length from head object");

        let content = rustfs_client.head_object().bucket(&app_config.rustfs_config.bucket_photos).key(photo_record.s3_key).send().await.map_err(|err| {
            tracing::error!("Error fetching head object");

           UploadsError::HeadObjectError(err.to_string())
        })?;

        match content.content_length() {
            Some(len) => {
                if len < MIN_PHOTO_SIZE_BYTES || len > MAX_PHOTO_SIZE_BYTES {
                    return Err(AppError::Uploads(UploadsError::InvalidFileSize("Uploaded item is not within our limits".to_string())));
                }
            },
            None => {
                return Err(AppError::Uploads(UploadsError::HeadObjectError("Error retriving content length".to_string())));
            }
        }

        let update_photo = sqlx::query!(r#"UPDATE photos SET status='processing', title=$2, description=$3, category=$4 WHERE id=$1 AND status='initiated'"#, photo_id, title.as_deref(), description.as_deref(), category.as_deref()).execute(connection).await.map_err(|_|{ tracing::error!("Error updating record for the upload");
            UploadsError::ErrorUpdatingPhoto(photo_id.to_string())
        })?;

        if update_photo.rows_affected() == 0 {
            let re_read_photo_record = sqlx::query!(r#"SELECT id, s3_key, status AS "status: PhotoStatus" FROM photos WHERE id = $1"#, photo_id).fetch_one(connection).await.map_err(|err: sqlx::Error| {
                tracing::error!("Can't find photo in our DB");

                match err {
                    sqlx::Error::RowNotFound => UploadsError::PhotoNotFound(photo_id.to_string()),
                    _ => UploadsError::PhotoQueryError("Error fetching photo".to_string())
                }
            })?;

            if re_read_photo_record.status != PhotoStatus::Initiated {
                return Ok(ConfirmUploadsResponse {
                    photo_id: *photo_id,
                    status: photo_record.status,
                });
            }
        }

        // notify on confirm ping worker to start derivatives generation
        notify_on_confirm.notify_one();

        let confirm_response = ConfirmUploadsResponse {
            photo_id: *photo_id,
            status: PhotoStatus::Processing
        };

        Ok(confirm_response)
    }
}

