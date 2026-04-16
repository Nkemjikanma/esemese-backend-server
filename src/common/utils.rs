use crate::common::errors::AppError;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(try_from = "String")]
pub struct ValidateString(String);

impl ValidateString {
    pub fn parse(s: String) -> Result<ValidateString, String> {
        let is_empty_or_whitespace = s.trim().is_empty();

        let is_too_long = s.graphemes(true).count() > 256;

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

        let contains_forbidden_chars = s.chars().any(|c| forbidden_characters.contains(&c));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_chars {
            Err(format!("{} is an invalid input", s))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for ValidateString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ValidateString {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

// Argon2 - Password hashing
pub struct PasswordUtils;

impl PasswordUtils {
    pub fn hash_password(password: String) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);

        let argon2 = Argon2::default();

        let password = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::PasswordHashingError)?
            .to_string();

        Ok(password)
    }

    pub fn verify(password: &str, hash: &str) -> bool {
        let password_hash = match PasswordHash::new(hash) {
            Ok(h) => h,
            Err(_) => return false,
        };

        Argon2::default()
            .verify_password(password.as_bytes(), &password_hash)
            .is_ok()
    }
}

pub struct JWT;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Claims {
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}

impl JWT {
    pub fn generate_token(
        username: &str,
        jwt_secret: &str,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let expiration = 3600;
        let now = Utc::now();
        let exp = (now + Duration::seconds(expiration)).timestamp() as usize;
        let iat = now.timestamp() as usize;

        let claims = Claims {
            username: username.to_string(),
            exp,
            iat,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret.as_bytes()),
        )
    }

    pub fn verify_token(
        token: &str,
        jwt_secret: &str,
    ) -> Result<Claims, jsonwebtoken::errors::Error> {
        let config_secret = jwt_secret.as_bytes();

        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(config_secret),
            &Validation::new(Algorithm::HS256),
        )?;

        Ok(decoded.claims)
    }
}
