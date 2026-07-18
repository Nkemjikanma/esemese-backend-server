use uuid::Uuid;
use crate::common::errors::{AppError, PhotosError};
use crate::types::common::{PaginatedResponse, Pagination};
use crate::types::photos::{Photo, PhotosQueryInfo};

pub struct PhotosService;

 impl PhotosService {
	 // get all photos - filterable based on category and collection
	 #[tracing::instrument(name = "get_all_photos", skip(pool))]
	 pub async fn get_all_photos(query: PhotosQueryInfo,
	                             pagination: Pagination,
	                             pool: &sqlx::PgPool) -> Result<PaginatedResponse<Photo>, AppError> {
		 let PhotosQueryInfo { collection, collection_slug, category } = query;

		 let page = pagination.page.max(1);
		 let limit = pagination.limit;
		 let offset = (page- 1) * limit; // how many rows to skip, so skipping 0 rows is fine

		 let all_photos = sqlx::query_as!(Photo, r#"SELECT p.id, p.title, p.description, p.category, p.featured,
		 p.blurhash, p.s3_key, p.created_at, p.updated_at FROM photos p
		                                WHERE p.status = 'ready'
		                                  AND ($1::text IS NULL OR p.category = $1)
		                                  AND ($2::uuid IS NULL OR EXISTS (
		                                  	SELECT 1 FROM collection_photos cp
		                                  	WHERE cp.photo_id = p.id AND cp.collection_id = $2))
		                               	  AND ($3::text IS NULL OR EXISTS (
		                               	  	SELECT 1 FROM collection_photos cp
		                               	  	JOIN collections c ON c.id = cp.collection_id
		                               	  	WHERE cp.photo_id = p.id AND c.slug = $3))
		                                ORDER BY p.created_at DESC, p.id DESC LIMIT $4 OFFSET $5"#,
			 category, collection, collection_slug, limit as i64, offset as i64) // id, DESC - for when 2 photos share
			 // the same timestamp
			 .fetch_all(pool)
			 .await
			 .map_err
		 (|e| {
			 tracing::error!("Failed to fetch photos: {:?}", e);

			 PhotosError::ErrorFetchingPhotos(e.to_string())
		 })?;

		 // let filtered_photos = all_photos
			//  .into_iter()
			//  .filter(|b| category.as_ref().is_none_or(|c| b.category == *c))
			//  .filter(|b| collection.as_ref().is_none_or(|c| b.collection == *c))
			//  .collect();

		 let total: i64 = sqlx::query_scalar!(r#"SELECT COUNT(*) FROM photos p
                WHERE status='ready'
                  AND ($1::text IS NULL OR category = $1)
                  AND ($2::uuid IS NULL OR EXISTS (
                  SELECT 1 FROM collection_photos cp
                  WHERE cp.photo_id = p.id AND cp.collection_id = $2))"#,
			 category, collection)
			 .fetch_one(pool)
			 .await.map_err(|e: sqlx::Error| {

			 tracing::error!("Error retriving the total number of photos: {:?}", e);

			 PhotosError::ErrorFetchingPhotos(e.to_string())
		 })?.unwrap_or(0);

		 // JOIN collection_photos cp ON cp.photo_id = p.id
		 let total_pages =  if limit == 0 {
			 0
		 } else {
			 ((total + limit as i64 - 1) / limit as i64) as u32
		 };

		 let response = PaginatedResponse {
			data: all_photos,
			 total,
			 page,
			 limit,
			total_pages
		 };

		 Ok(response)
	 }

	 pub async fn get_photo_by_id(photo_id: Uuid, pool: &sqlx::PgPool) -> Result<Photo, AppError> {
		let photo = sqlx::query_as!(Photo, r#"SELECT id, title, description, category, featured, blurhash,
		 s3_key, created_at, updated_at FROM photos WHERE id = $1"#, photo_id)
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