use crate::common::errors::ConfigError;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub application: ApplicationConfig,
    pub database: DBConfig,
    pub rustfs_config: RustFSConfig
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApplicationConfig {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub admin_username: SecretString,
    pub admin_password_hash: SecretString,
    pub jwt_secret: SecretString,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DBConfig {
    pub username: String,
    pub password: SecretString,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DBConfig {
    pub fn connection_option(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Disable
        };

        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(&self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
            .database(&self.database_name)
    }
}


#[derive(Deserialize, Debug, Clone)]
pub struct RustFSConfig {
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub endpoint_internal: String, // server → RustFS, http://rustfs:9000 in prod
    pub endpoint_public: String, // browser → RustFS, https://s3.esemese.xyz in prod
    pub bucket_photos: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "development" => Ok(Self::Development),
            "production" => Ok(Self::Production),
            other => Err(format!("Unknown environment: {}", other)),
        }
    }
}

impl Config {
    pub fn load_config() -> Result<Self, ConfigError> {
        let env: Environment = std::env::var("APP_ENV")
            .unwrap_or_else(|_| "development".to_string())
            .try_into()
            .map_err(|e: String| ConfigError::InvalidEnv(e))?;

        match env {
            Environment::Development => Self::development_config(),
            Environment::Production => Self::production_config(),
        }
    }

    fn production_config() -> Result<Self, ConfigError> {
        let application = ApplicationConfig {
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8000".to_string())
                .parse()
                .map_err(|_| {
                    ConfigError::InvalidEnv("PORT must be valid port number".to_string())
                })?,

            host: "0.0.0.0".to_string(),
            admin_username: SecretString::from(std::env::var("ADMIN_USERNAME").map_err(|_| {
                ConfigError::MissingEnv("Admin username not configured".to_string())
            })?),
            admin_password_hash: SecretString::from(std::env::var("ADMIN_PASSWORD_HASH").map_err(
                |_| ConfigError::MissingEnv("Admin password hash can't be configuted".to_string()),
            )?),
            jwt_secret: SecretString::from(
                std::env::var("JWT_SECRET")
                    .map_err(|_| ConfigError::MissingEnv("JWT not configured".to_string()))?,
            ),
        };

        let database = DBConfig {
            username: std::env::var("DB_USERNAME")
                .map_err(|_| ConfigError::MissingEnv("DB_USERNAME".to_string()))?,
            password: SecretString::from(
                std::env::var("DB_PASSWORD")
                    .map_err(|_| ConfigError::MissingEnv("DB_PASSWORD".to_string()))?,
            ),
            port: std::env::var("DB_PORT")
                .map_err(|_| ConfigError::MissingEnv("DB_PORT".to_string()))?
                .parse()
                .map_err(|e| ConfigError::InvalidEnv("Port hast to be a number".to_string()))?,
            host: std::env::var("DB_HOST")
                .map_err(|_| ConfigError::MissingEnv("DB_HOST".to_string()))?,
            database_name: std::env::var("DB_NAME")
                .map_err(|e| ConfigError::MissingEnv("DB_NAME".to_string()))?,
            require_ssl: true,
        };

        let rustfs_config = RustFSConfig {
            region: std::env::var("RUSTFS_REGION").map_err(|_| ConfigError::MissingEnv("RUSTFS_REGION".to_string()))?,
            access_key_id: std::env::var("RUSTFS_ACCESS_KEY_ID").map_err(|_| ConfigError::MissingEnv("RUSTFS_ACCESS_KEY_ID".to_string()))?,
            secret_access_key: std::env::var("RUSTFS_SECRET_ACCESS_KEY").map_err(|_| ConfigError::MissingEnv("RUSTFS_SECRET_ACCESS_KEY".to_string()))?,
            endpoint_public: std::env::var("RUSTFS_ENDPOINT_PUBLIC").map_err(|_| ConfigError::MissingEnv("RUSTFS_ENDPOINT_PUBLIC".to_string()))?,
            endpoint_internal: std::env::var("RUSTFS_ENDPOINT_INTERNAL").map_err(|_| ConfigError::MissingEnv("RUSTFS_ENDPOINT_INTERNAL".to_string()))?,
            bucket_photos: std::env::var("RUSTFS_BUCKET_PHOTOS").map_err(|_| ConfigError::MissingEnv("RUSTFS_BUCKET_PHOTOS".to_string()))?,
        };

        Ok(Self {
            application,
            database,
           rustfs_config
        })
    }
    fn development_config() -> Result<Self, ConfigError> {
        let application = ApplicationConfig {
            port: 8000,
            host: "127.0.0.1".to_string(),
            admin_username: SecretString::from("nkemjika".to_string()),
            admin_password_hash: SecretString::from(
                "$argon2d$v=19$m=12,t=3,p=1$eG9xb3kwMW13MDgwMDAwMA$C9GxzXC91CKUL79kOtEKrA"
                    .to_string(),
            ),
            jwt_secret: SecretString::from("jwt_secret".to_string()),
        };

        let database = DBConfig {
            username: "nkemjika".to_string(),
            password: SecretString::from("password".to_string()),
            port: 5432,
            database_name: "esemese_db".to_string(),
            host: "127.0.0.1".to_string(),
            require_ssl: false,
        };

        let dev_rustfs_config = RustFSConfig {
            region: std::env::var("RUSTFS_REGION").map_err(|_| ConfigError::MissingEnv("RUSTFS_REGION".to_string()))?,
            access_key_id: std::env::var("RUSTFS_ACCESS_KEY_ID").map_err(|_| ConfigError::MissingEnv("RUSTFS_ACCESS_KEY_ID".to_string()))?,
            secret_access_key: std::env::var("RUSTFS_SECRET_ACCESS_KEY").map_err(|_| ConfigError::MissingEnv("RUSTFS_SECRET_ACCESS_KEY".to_string()))?,
            endpoint_public: std::env::var("RUSTFS_ENDPOINT_PUBLIC").map_err(|_| ConfigError::MissingEnv("RUSTFS_ENDPOINT_PUBLIC".to_string()))?,
            endpoint_internal: std::env::var("RUSTFS_ENDPOINT_INTERNAL").map_err(|_| ConfigError::MissingEnv("RUSTFS_ENDPOINT_INTERNAL".to_string()))?,
            bucket_photos: std::env::var("RUSTFS_BUCKET_PHOTOS").map_err(|_| ConfigError::MissingEnv("RUSTFS_BUCKET_PHOTOS".to_string()))?,
        };

        Ok(Self {
            application,
            database,
            rustfs_config: dev_rustfs_config,
        })
    }
}
