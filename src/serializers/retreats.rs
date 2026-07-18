use crate::{
    entities_helper::{RetreatModel},
    serializers::pagination::Paginate,
    utils::serializer::deserialize_some,
};
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
    pub thumbnail_image: Option<String>,
    pub banner_image: Option<String>,
    pub average_rating: Option<f64>,
}

impl From<RetreatModel> for ReadRetreatSerializer {
    fn from(value: RetreatModel) -> Self {
        ReadRetreatSerializer {
            retreat_id: value.retreat_id,
            name: value.name,
            description: value.description,
            category_id: value.category_id,
            slug: value.slug,
            social_links: value.social_links,
            email: value.email,
            phone: value.phone,
            latitude: value.latitude,
            longitude: value.longitude,
            address: value.address,
            budget_min: value.budget_min,
            budget_max: value.budget_max,
            is_published: value.is_published,
            thumbnail_image: value.thumbnail_image.map(|_| format!("/retreats/{}/thumbnail/image/", value.retreat_id)),
            banner_image: value.banner_image.map(|_| format!("/retreats/{}/banner/image/", value.retreat_id)),
            average_rating: None,
        }
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

#[derive(Debug, Clone, Deserialize)]
pub struct RetreatFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub is_published: Option<bool>,
    pub search: Option<String>,
    pub category_id: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

impl Paginate for RetreatFilter {
    fn limit(&self) -> u64 {
        self.page_size.unwrap_or(10)
    }

    fn page(&self) -> u64 {
        self.page.unwrap_or(1)
    }

    fn offset(&self) -> u64 {
        let page: u64 = self.page();
        if page == 0 {
            return 0;
        }
        (page - 1) * self.limit()
    }
}
