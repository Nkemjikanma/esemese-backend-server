use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::types::common::PaginatedResponse;

#[derive(Debug, Deserialize)]
pub struct PhotosQueryInfo {
	pub collection: Option<Uuid>,
	pub collection_slug: Option<String>,
	pub category: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Photo {
	pub id: Uuid,
	pub title: Option<String>,
	pub description: Option<String>,
	pub category: Option<String>,
	pub featured: bool,
	pub blurhash: Option<String>,
	pub s3_key: String,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

pub type Photos = Vec<Photo>;