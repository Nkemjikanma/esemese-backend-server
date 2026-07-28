use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Variant {
    pub id: Uuid,
    pub photo_id: Uuid,
    pub s3_key: String,
}

pub type Variants = Vec<Variant>;
