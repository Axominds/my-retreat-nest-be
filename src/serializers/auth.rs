use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct LoginSerializer{
    pub email: String,
    pub password: String
}

#[derive(Debug, Serialize, Clone)]
pub struct LoginResponseSerializer{
    pub access_token: String,
    pub refresh_token: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaim{
    pub user_id: i64,
    pub email: String,
    pub name: String,
}


#[derive(Debug, Clone, Deserialize, Validate)]
pub struct RefreshSerializer{
    pub refresh_token: String
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ForgotPasswordSerializer {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ResetPasswordSerializer {
    pub token: String,
    pub new_password: String,
}