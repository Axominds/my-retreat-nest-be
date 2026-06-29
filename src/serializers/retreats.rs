use crate::{entities_helper::{RetreatModel}, map_fields, utils::serializer::deserialize_some};
use sea_orm::prelude::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateRetreatSerializer {
    pub name: String,
    pub description: Option<String>,
    pub category_id: i64,
    pub slug: String,
    pub social_links: JsonValue,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub address: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ReadRetreatSerializer {
    retreat_id: i64,
    name: String,
    description: Option<String>,
    category_id: i64,
    slug: String,
    social_links: JsonValue,
    email: Option<String>,
    phone: Option<String>,
    latitude: Option<Decimal>,
    longitude: Option<Decimal>,
    address: Option<String>,
    budget_min: Option<Decimal>,
    budget_max: Option<Decimal>,
    is_published: bool,
}

impl From<RetreatModel> for ReadRetreatSerializer {
    fn from(value: RetreatModel) -> Self {
        map_fields!(value, ReadRetreatSerializer, {
            retreat_id,
            name,
            description,
            category_id,
            slug,
            social_links,
            email,
            phone,
            latitude,
            longitude,
            address,
            budget_min,
            budget_max,
            is_published
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct UpdateRetreatSerializer {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub description: Option<Option<String>>,
    pub category_id: Option<i64>,
    pub slug: Option<String>,
    pub social_links: Option<JsonValue>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub email: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub phone: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub latitude: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub longitude: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub address: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub budget_min: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub budget_max: Option<Option<Decimal>>,
    pub is_published: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateRetreatUserSerializer {
    pub name: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct UpdateRetreatUserSerializer {
    #[serde(default, deserialize_with = "deserialize_some")]
    pub role: Option<Option<String>>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ReadRetreatUserSerializer {
    pub retreat_user_id: i64,
    pub retreat_id: i64,
    pub user_id: i64,
    pub name: String,
    pub email: String,
    pub role: Option<String>,
}
