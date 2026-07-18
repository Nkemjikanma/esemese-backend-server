use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CollectionsQueryInfo {
	pub id: Option<Uuid>,
	pub slug: Option<String>
}

#[derive(Serialize)]
pub struct Collection {
	pub id: Uuid,
	pub name: String,
	pub slug: String,
	pub description: Option<String>,
	pub cover_photo_id: Option<Uuid>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub cover_s3_key: Option<String>,
	pub cover_blurhash: Option<String>,
}

pub type Collections = Vec<Collection>;