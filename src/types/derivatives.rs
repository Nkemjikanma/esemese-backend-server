use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct PhotoMetadata {
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<i32>,
    pub aperture: Option<String>,        // "f/2.8"
    pub shutter_speed: Option<String>,   // "1/250"
    pub focal_length: Option<String>,    // "50 mm"
    pub taken_at: Option<DateTime<Utc>>,
}