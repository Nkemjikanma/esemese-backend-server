use crate::common::errors::{AppError, CollectionError, PhotosError};
use crate::types::collections::{Collection, CollectionsQueryInfo};
use crate::types::common::{PaginatedResponse, Pagination};

pub struct CollectionsService;

impl CollectionsService {
	// list of all collections, ordered by date.
	#[tracing::instrument(name = "get_all_collections", skip(pool))]
	pub async fn get_all_collections(pagination: Pagination,
	                                 pool: &sqlx::PgPool) -> Result<PaginatedResponse<Collection>, AppError> {

		let page = pagination.page.max(1);
		let limit = pagination.limit;
		let offset = ((page - 1) * limit) as i64;

		// sort order
		let all_collections = sqlx::query_as!(Collection, r#"SELECT c.id AS "id!",
       															c.name        AS "name!",
         														c.slug        AS "slug!",
         														c.description,            -- nullable because struct is Option
         														c.cover_photo_id,         -- nullable because struct is Option
         														c.created_at  AS "created_at!",
         														c.updated_at  AS "updated_at!",
         														ph.s3_key     AS "cover_s3_key?",
         														ph.blurhash   AS "cover_blurhash?"
       											FROM collections c
         										LEFT JOIN photos ph ON ph.id = c.cover_photo_id
       											ORDER BY c.created_at DESC, c.id DESC
                                      LIMIT $1 OFFSET $2"#, limit as i64, offset)
			.fetch_all(pool)
			.await
			.map_err(|e: sqlx::Error| {
			tracing::error!("Failed to fetch collections: {:?}", e);

			PhotosError::ErrorFetchingCollections(e.to_string())
		})?;

		let total: i64 = sqlx::query_scalar!(r#"SELECT COUNT(*) FROM collections"#)
			.fetch_one(pool)
			.await
			.map_err(|e| {
				tracing::error!("Error retrieving the total number of collections: {:?}", e);

				PhotosError::ErrorFetchingCollections(e.to_string())
		})?.unwrap_or(0);

		let total_pages = if limit == 0 {
			0
		} else {
			((total + limit as i64 - 1) / limit as i64) as u32 // TODO: WHY THIS?
		};

		let response = PaginatedResponse {
			data: all_collections,
			total,
			page,
			limit,
			total_pages
		};

		Ok(response)
	}

	#[tracing::instrument(name = "get_collection_by_slug", skip(pool))]
	pub async fn get_collection_by_slug(slug: String,
	                                    pool: &sqlx::PgPool) -> Result<Collection, AppError>{

		let collection = sqlx::query_as!(Collection, r#"SELECT c.id AS "id!",
       												 			c.name        AS "name!",
         														c.slug        AS "slug!",
         														c.description,            -- nullable because struct is Option
         														c.cover_photo_id,         -- nullable because struct is Option
         														c.created_at  AS "created_at!",
         														c.updated_at  AS "updated_at!",
         														ph.s3_key     AS "cover_s3_key?",
         														ph.blurhash   AS "cover_blurhash?"
														FROM collections c
										 				LEFT JOIN photos ph ON ph.id = c.cover_photo_id
														WHERE slug = $1"#, slug)
			.fetch_one(pool)
			.await.map_err(|e: sqlx::Error| {

			tracing::error!("Failed to fetch collection with slug - {}: {:?}", slug, e);

			match e {
				sqlx::Error::RowNotFound => CollectionError::CollectionNotFound(e.to_string()),
				_ => CollectionError::CollectionQueryError(e.to_string())
			}
		})?;

		Ok(collection)
	}
}