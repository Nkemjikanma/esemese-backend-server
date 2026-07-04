use actix_web::App;
use uuid::Uuid;
use crate::common::errors::{AppError, PhotosError};
use crate::types::photos::{Photo, Photos, PhotosQueryInfo};

pub struct PhotosService;

 impl PhotosService {
	 // get all photos - filterable based on category and collection
	 #[tracing::instrument(name = "get_all_photos", skip(pool))]
	 pub async fn get_all_photos(query: PhotosQueryInfo, pool: &sqlx::PgPool) -> Result<Photos, AppError> {
		 let PhotosQueryInfo { collection, category } = query;

		 let all_photos = sqlx::query_as!(Photo, r#"SELECT * FROM photos WHERE status='done'"#).fetch_all(pool)
			 .await
			 .map_err
		 (|e| {
			 tracing::error!("Failed to fetch photos: {:?}", e);

			 PhotosError::ErrorFetchingPhotos(e.to_string())
		 })?;

		 let filtered_photos = all_photos
			 .into_iter()
			 .filter(|b| category.as_ref().is_none_or(|c| b.category == *c))
			 .filter(|b| collection.as_ref().is_none_or(|c| b.collection == *c))
			 .collect();

		 Ok(filtered_photos)
	 }

	 pub async fn get_photo_by_id(photo_id: Uuid, pool: &sqlx::PgPool) -> Result<Photo, AppError> {
		let photo = sqlx::query_as!(Photo, r#"SELECT * FROM photos WHERE photo_id = $1"#, photo_id)
			.fetch_one(pool)
			.await
			.map_err(|e: sqlx::Error| {
			tracing::error!("Failed to fetch photo with Id, {}: {:?}", photo_id, e);

			match e {
				sqlx::Error::RowNotFound => PhotosError::PhotoNotFound(e.to_string()),
				_ => PhotosError::PhotoQueryError(e.to_string())
			}
		})?;

		 Ok(photo)
	 }
 }