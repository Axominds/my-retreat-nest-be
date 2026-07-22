use crate::{
    entities_helper::ListingRequestModel,
    serializers::pagination::Paginate,
};
use sea_orm::prelude::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateListingRequestSerializer {
    pub owner_name: String,
    #[validate(email)]
    pub owner_email: String,
    pub owner_phone: Option<String>,
    pub retreat_name: String,
    pub retreat_description: Option<String>,
    pub category_id: i64,
    pub retreat_email: Option<String>,
    pub retreat_phone: Option<String>,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub address: Option<String>,
    pub budget_min: Option<Decimal>,
    pub budget_max: Option<Decimal>,
    pub social_links: JsonValue,
}

#[derive(Serialize, Debug, Clone)]
pub struct ReadListingRequestSerializer {
    pub listing_request_id: i64,
    pub owner_name: String,
    pub owner_email: String,
    pub owner_phone: Option<String>,
    pub retreat_name: String,
    pub retreat_description: Option<String>,
    pub category_id: i64,
    pub retreat_email: Option<String>,
    pub retreat_phone: Option<String>,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub address: Option<String>,
    pub budget_min: Option<Decimal>,
    pub budget_max: Option<Decimal>,
    pub social_links: JsonValue,
    pub status: String,
    pub reviewed_by: Option<i64>,
    pub reviewed_at: Option<String>,
    pub rejection_reason: Option<String>,
    pub retreat_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ListingRequestModel> for ReadListingRequestSerializer {
    fn from(value: ListingRequestModel) -> Self {
        ReadListingRequestSerializer {
            listing_request_id: value.listing_request_id,
            owner_name: value.owner_name,
            owner_email: value.owner_email,
            owner_phone: value.owner_phone,
            retreat_name: value.retreat_name,
            retreat_description: value.retreat_description,
            category_id: value.category_id,
            retreat_email: value.retreat_email,
            retreat_phone: value.retreat_phone,
            latitude: value.latitude,
            longitude: value.longitude,
            address: value.address,
            budget_min: value.budget_min,
            budget_max: value.budget_max,
            social_links: value.social_links,
            status: value.status,
            reviewed_by: value.reviewed_by,
            reviewed_at: value.reviewed_at.map(|d| d.to_string()),
            rejection_reason: value.rejection_reason,
            retreat_id: value.retreat_id,
            created_at: value.created_at.to_string(),
            updated_at: value.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListingRequestFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

impl Paginate for ListingRequestFilter {
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

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ApproveListingRequestSerializer {
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RejectListingRequestSerializer {
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateListingRequestSerializer {
    pub owner_name: Option<String>,
    pub owner_email: Option<String>,
    pub owner_phone: Option<String>,
    pub retreat_name: Option<String>,
    pub retreat_description: Option<String>,
    pub category_id: Option<i64>,
    pub retreat_email: Option<String>,
    pub retreat_phone: Option<String>,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub address: Option<String>,
    pub budget_min: Option<Decimal>,
    pub budget_max: Option<Decimal>,
    pub social_links: Option<JsonValue>,
}
