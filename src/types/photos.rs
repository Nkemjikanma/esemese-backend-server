use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct PhotosQueryInfo {
	pub collection: Option<String>,
	pub category: Option<String>
}

#[derive(Debug, Serialize)]
pub struct Photo {
	pub id: Uuid,
	pub title: String,
	pub description: String,
	pub category: String,
	pub featured: bool,
	pub blurhash: String,
	pub s3_key: String,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

pub type Photos = Vec<Photo>;