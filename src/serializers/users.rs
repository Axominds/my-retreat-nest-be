use std::borrow::Cow;

use crate::{entities::users::Model as UserModel, serializers::pagination::Paginate, utils::serializer::deserialize_some};
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_phone(phone: &str) -> Result<(), ValidationError> {
    if phone.len() < 9 {
        return Err(ValidationError::new("Validation").with_message(Cow::from("Invalid phone number")))
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateUserSerializer{
    pub name: String,
    #[validate(email)]
    pub email: String,
    pub password: String,
    #[validate(custom(function="validate_phone"))]
    pub phone: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ReadUserSerializer{
    user_id: i64,
    name: String,
    email: String,
    phone: Option<String>

}

impl From<UserModel> for ReadUserSerializer{
    fn from(value: UserModel) -> Self {
        ReadUserSerializer { user_id: value.user_id, name: value.name, email: value.email, phone: value.phone }
    }
}


#[derive(Debug, Clone, Deserialize)]
pub struct UserFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

impl Paginate for UserFilter {
    fn limit(&self) -> u64 {
        self.page_size.unwrap_or(10)
    }

    fn page(&self) -> u64 {
        self.page.unwrap_or(1)
    }

    fn offset(&self) -> u64 {
        let page = self.page();
        if page == 0 {
            return 0;
        }
        (page - 1) * self.limit()
    }
}

#[derive(Debug, Clone, Deserialize, Validate, Serialize)]
pub struct UpdateUserSerializer{
    pub name: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(custom(function="validate_phone"))]
    #[serde(default, deserialize_with = "deserialize_some")]
    pub phone: Option<Option<String>>,
}