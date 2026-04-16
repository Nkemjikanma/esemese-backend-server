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

#[derive(Error, Debug)]
pub enum AppError {
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

            AppError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
