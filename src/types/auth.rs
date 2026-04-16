use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}
