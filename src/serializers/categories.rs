use std::borrow::Cow;

use crate::{entities_helper::categories::CategoryModel, map_fields, utils::serializer::deserialize_some};
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_phone(phone: &str) -> Result<(), ValidationError> {
    if phone.len() < 9 {
        return Err(ValidationError::new("Validation").with_message(Cow::from("Invalid name")));
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateCategorySerializer {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ReadCategorySerializer {
    category_id: i64,
    name: String,
    description: Option<String>,
}

impl From<CategoryModel> for ReadCategorySerializer {
    fn from(value: CategoryModel) -> Self {
        map_fields!(value, ReadCategorySerializer, {
            category_id,
            name,
            description
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct UpdateCategorySerializer {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub description: Option<Option<String>>,
}
