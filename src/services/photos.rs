use crate::common::errors::{AppError, DerivativesError, PhotosError};
use crate::types::common::{PaginatedResponse, Pagination};
use crate::types::photos::UpdatePhotoMetadata;
use crate::types::{
    app::AppState,
    photos::{Photo, PhotosQueryInfo},
    variants::Variant,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PhotosService;

impl PhotosService {
    // get all photos - filterable based on category and collection
    #[tracing::instrument(name = "get_all_photos", skip(pool))]
    pub async fn get_all_photos(
        query: PhotosQueryInfo,
        pagination: Pagination,
        pool: &sqlx::PgPool,
    ) -> Result<PaginatedResponse<Photo>, AppError> {
        let PhotosQueryInfo {
            collection,
            collection_slug,
            category,
        } = query;

        let page = pagination.page.max(1);
        let limit = pagination.limit;
        let offset = (page - 1) * limit; // how many rows to skip, so skipping 0 rows is fine

        let all_photos = sqlx::query_as!(
            Photo,
            r#"SELECT p.id, p.title, p.description, p.category, p.featured,
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
            category,
            collection,
            collection_slug,
            limit as i64,
            offset as i64
        ) // id, DESC - for when 2 photos share
        // the same timestamp
        .fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch photos: {:?}", e);

            PhotosError::ErrorFetchingPhotos(e.to_string())
        })?;

        // let filtered_photos = all_photos
        //  .into_iter()
        //  .filter(|b| category.as_ref().is_none_or(|c| b.category == *c))
        //  .filter(|b| collection.as_ref().is_none_or(|c| b.collection == *c))
        //  .collect();

        let total: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM photos p
                WHERE status='ready'
                  AND ($1::text IS NULL OR category = $1)
                  AND ($2::uuid IS NULL OR EXISTS (
                  SELECT 1 FROM collection_photos cp
                  WHERE cp.photo_id = p.id AND cp.collection_id = $2))"#,
            category,
            collection
        )
        .fetch_one(pool)
        .await
        .map_err(|e: sqlx::Error| {
            tracing::error!("Error retriving the total number of photos: {:?}", e);

            PhotosError::ErrorFetchingPhotos(e.to_string())
        })?
        .unwrap_or(0);

        // JOIN collection_photos cp ON cp.photo_id = p.id
        let total_pages = if limit == 0 {
            0
        } else {
            ((total + limit as i64 - 1) / limit as i64) as u32
        };

        let response = PaginatedResponse {
            data: all_photos,
            total,
            page,
            limit,
            total_pages,
        };

        Ok(response)
    }

    pub async fn get_photo_by_id(photo_id: Uuid, pool: &sqlx::PgPool) -> Result<Photo, AppError> {
        let photo = sqlx::query_as!(
            Photo,
            r#"SELECT id, title, description, category, featured, blurhash,
		 s3_key, created_at, updated_at FROM photos WHERE id = $1"#,
            photo_id
        )
        .fetch_one(pool)
        .await
        .map_err(|e: sqlx::Error| {
            tracing::error!("Failed to fetch photo with Id, {}: {:?}", photo_id, e);

            match e {
                sqlx::Error::RowNotFound => PhotosError::PhotoNotFound(photo_id.to_string()),
                _ => PhotosError::PhotoQueryError(e.to_string()),
            }
        })?;

        Ok(photo)
    }

    pub async fn update_photo_metadata(
        photo_item_id: Uuid,
        metadata: UpdatePhotoMetadata,
        connection: &sqlx::PgPool,
    ) -> Result<(), AppError> {
        // update the records with new fields using UPDATE and COALESCE
        let photo_update_query = sqlx::query!(
            r#"UPDATE photo_metadata 
            SET
                aperture = COALESCE($1, aperture),
                camera = COALESCE($2, camera),
                focal_length = COALESCE($3, focal_length),
                iso = COALESCE($4, iso), 
                shutter_speed = COALESCE($5, shutter_speed),
                lens = COALESCE($6, lens), 
                location = COALESCE($7, location), 
                taken_at = COALESCE($8, taken_at)
            WHERE photo_id = $9"#,
            metadata.aperture,
            metadata.camera,
            metadata.focal_length,
            metadata.iso,
            metadata.shutter_speed,
            metadata.lens,
            metadata.location,
            metadata.taken_at,
            photo_item_id
        )
        .execute(connection)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to update photo metadata");

            PhotosError::ErrorUpdatingPhotoMetadata
        })?;

        if photo_update_query.rows_affected() == 0 {
            return Err(AppError::Photos(PhotosError::PhotoNotFound(
                photo_item_id.to_string(),
            )));
        }

        Ok(())
    }

    pub async fn delete_photo(photo_id: Uuid, app_state: &AppState) -> Result<(), AppError> {
        let AppState {
            connection,
            rustfs_client,
            app_config,
            notify_on_confirm,
        } = app_state;
        // get the phot item with s3_key and blurhash
        let photo_item = Self::get_photo_by_id(photo_id, connection).await?;

        // get variant s3_keys
        let photo_variants = sqlx::query_as!(
            Variant,
            r#"SELECT id, photo_id, s3_key FROM variants WHERE
		 photo_id =
		 $1"#,
            photo_id
        )
        .fetch_all(connection)
        .await
        .map_err(|e| {
            tracing::error!("Error retrieving photo variants for deletion");

            PhotosError::ErrorFetchingPhotos(e.to_string())
        })?;

        // Update photo status to deleting before touching the bucket
        let update_photo = sqlx::query!(
            r#"UPDATE photos SET status = 'deleting', updated_at = now() WHERE id = $1
		AND status = 'ready'"#,
            photo_id
        )
        .execute(connection)
        .await
        .map_err(|e| {
            tracing::error!("Error updating photo status to deleting in preparation for deletions");

            PhotosError::ErrorUpdatingPhoto(photo_id.to_string())
        })?;

        if update_photo.rows_affected() == 0 {
            return Err(AppError::Photos(PhotosError::ErrorUpdatingPhoto(
                "No rows were updated during deletion \
			process"
                    .to_string(),
            )));
        }
        /*
         * Clear the bucket first because the cleanup job still happens in the background.
         * So any orphaned photos row gets cleaned up eventually - add deleted to status so that sweeper can clean up .
         */
        // Delete variants from bucket
        for photo_variant in photo_variants {
            rustfs_client
                .delete_object()
                .bucket(&app_config.rustfs_config.bucket_photos)
                .key(&photo_variant.s3_key)
                .send()
                .await
                .map_err(|e| {
                    tracing::error!(
                        "There has been an error trying to delete variant {:?}",
                        photo_variant.id
                    );

                    DerivativesError::DeletionError(e.to_string())
                })?;
        }

        // delete photo in bucket
        rustfs_client
            .delete_object()
            .bucket(&app_config.rustfs_config.bucket_photos)
            .key(&photo_item.s3_key)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Error deleting photo from s3 bucket");

                PhotosError::ErrorDeletingPhoto(e.to_string())
            })?;

        // Delete the photo and cascade to metadata, variants
        sqlx::query!(r#"DELETE FROM photos WHERE id = $1"#, photo_item.id)
            .execute(connection)
            .await
            .map_err(|e| {
                tracing::error!("Error deleting photo: {:?}", e);
                PhotosError::ErrorDeletingPhoto(e.to_string())
            })?;

        Ok(())
    }
}
