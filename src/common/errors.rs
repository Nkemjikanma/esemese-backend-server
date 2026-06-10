use actix_web::{
    HttpResponse,
    error::ResponseError,
    http::{StatusCode, header::ContentType},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnv(String),

    #[error("Invalid environment: {0}")]
    InvalidEnv(String),
}
#[derive(Debug, Error)]
pub enum UploadsError{
    #[error("There has been an error trying to generate the presinged URI : {0}")]
    PresignedURIGenerationError(String),

    #[error("File to small or too large. Upload file within the limit: {0}")]
    InvalidFileSize(String),

    #[error("Error creating new photo item for initiating upload: {0}")]
    UploadInitiationError(String),

    #[error("Photo with id {0} not found in our database")]
    PhotoNotFound(String),

    #[error("Error retrieving Photo with id {0} from our database")]
    PhotoQueryError(String),

    #[error("There was an error retrieving the head object from bucket: {0}")]
    HeadObjectError(String),

    #[error("There was an error updating photo records: {0}")]
    ErrorUpdatingPhoto(String),
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    Uploads(#[from] UploadsError),

    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("There's something wrong with you input: {0}")]
    InputValidationError(String),

    #[error("Invalid login credentials provided")]
    InvalidUserCredentials,

    #[error("Error hashing password")]
    PasswordHashingError,

    #[error("JWT creation failed")]
    JWTCreationFailed,

    #[error("Invalid token provided to validator")]
    InvalidToken,

    #[error("Invalid Mime type detected at extractor: {0}")]
    InvalidContentType(String),
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let message = self.to_string();

        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(serde_json::json!({"error": message}))
    }

    fn status_code(&self) -> StatusCode {
        match self {
            AppError::PasswordHashingError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InvalidUserCredentials => StatusCode::UNAUTHORIZED,
            AppError::JWTCreationFailed => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InvalidToken => StatusCode::UNAUTHORIZED,
            AppError::InputValidationError(_) => StatusCode::BAD_REQUEST,
            AppError::InvalidContentType(_)=> StatusCode::BAD_REQUEST,

            AppError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,


            AppError::Uploads(UploadsError::PresignedURIGenerationError(_)) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Uploads(UploadsError::InvalidFileSize(_)) => StatusCode::BAD_REQUEST,
            AppError::Uploads(UploadsError::UploadInitiationError(_)) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Uploads(UploadsError::PhotoNotFound(_)) => StatusCode::NOT_FOUND,
            AppError::Uploads(UploadsError::PhotoQueryError(_)) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Uploads(UploadsError::HeadObjectError(_)) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Uploads(UploadsError::ErrorUpdatingPhoto(_)) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
