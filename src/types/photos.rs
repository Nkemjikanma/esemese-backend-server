use crate::types::common::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::panic::Location;
use uuid::Uuid;

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

#[derive(Debug, Serialize)]
pub struct PhotoMetadata {
    pub id: Uuid,
    pub photo_id: Uuid,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<i32>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub focal_length: Option<String>,
    pub location: Option<String>,
    pub taken_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePhotoMetadata {
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<i32>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub focal_length: Option<String>,
    pub location: Option<String>,
    pub taken_at: Option<DateTime<Utc>>,
}
