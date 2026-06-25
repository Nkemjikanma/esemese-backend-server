use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug)]
pub struct InitiateUploadsRequest{
    pub file_name: String,
    pub content_type: ContentType,
    pub file_size: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentType {
    #[serde(rename = "image/jpeg")]
    ImageJpeg,
    #[serde(rename = "image/png")]
    ImagePng,
    #[serde(rename = "image/webp")]
    ImageWebp
}

impl ContentType {
    pub fn mime(&self) -> &'static str {
        match self {
            ContentType::ImageJpeg => "image/jpeg",
            ContentType::ImagePng => "image/png",
            ContentType::ImageWebp => "image/webp"
        }
    }

   pub fn extension(&self) -> &str {
        match self {
            ContentType::ImageJpeg => "jpeg",
            ContentType::ImagePng => "png",
            ContentType::ImageWebp => "webp"
        }
    }
}

#[derive(Serialize, Debug)]
pub struct InitiateUploadResponse {
    pub photo_id: Uuid,
    pub uri: String,
}


#[derive(Serialize, Debug, sqlx::Type, PartialEq, Copy, Clone)]
#[sqlx(type_name = "photo_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PhotoStatus {
    Initiated,
    Uploaded,
    Processing,
    Ready,
    Failed
}

#[derive(Deserialize, Debug)]
pub struct ConfirmUploadsRequest{
    pub photo_id: Uuid,
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct ConfirmUploadsResponse {
   pub photo_id: Uuid,
    pub status: PhotoStatus,
}
