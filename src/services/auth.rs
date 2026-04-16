use crate::{
    common::{
        errors::AppError,
        utils::{JWT, PasswordUtils, ValidateString},
    },
    config::ApplicationConfig,
    types::auth::LoginForm,
};
use secrecy::ExposeSecret;
use sqlx::PgPool;

pub struct AuthServices;

impl AuthServices {
    #[tracing::instrument(name = "login", skip(app_state))]
    pub async fn login(
        login_form: LoginForm,
        app_state: &ApplicationConfig,
    ) -> Result<String, AppError> {
        let LoginForm { username, password } = login_form;

        let validated_username =
            ValidateString::parse(username).map_err(|e| AppError::InputValidationError(e))?;

        let validated_password = ValidateString::parse(password)
            .map_err(|e| AppError::InputValidationError(e))?
            .as_ref()
            .to_string();

        if validated_username.as_ref().to_lowercase().to_string()
            != app_state
                .admin_username
                .expose_secret()
                .to_lowercase()
                .to_string()
        {
            tracing::error!("Invalid login attempt");

            return Err(AppError::InvalidUserCredentials);
        }

        if !PasswordUtils::verify(
            &validated_password,
            app_state.admin_password_hash.expose_secret(),
        ) {
            tracing::error!("Invalid login attempt");

            return Err(AppError::InvalidUserCredentials);
        }

        tracing::info!("Login successful");

        // generate jwt
        let token = JWT::generate_token(
            &validated_username.as_ref(),
            &app_state.jwt_secret.expose_secret(),
        )
        .map_err(|e| {
            tracing::error!("Error generating JWT");

            AppError::JWTCreationFailed
        })?;

        Ok(token)
    }
}
