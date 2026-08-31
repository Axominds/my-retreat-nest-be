use crate::{entities_helper::amenities::AmenityModel, serializers::pagination::Paginate};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateAmenitySerializer {
    pub label: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ReadAmenitySerializer {
    pub amenity_id: i64,
    pub label: String,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct UpdateAmenitySerializer {
    pub label: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AmenityFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub search: Option<String>,
}

impl Paginate for AmenityFilter {
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

impl From<AmenityModel> for ReadAmenitySerializer {
    fn from(value: AmenityModel) -> Self {
        ReadAmenitySerializer {
            amenity_id: value.amenity_id,
            label: value.label,
            created_at: value.created_at.to_string(),
            updated_at: value.updated_at.to_string(),
            created_by: value.created_by,
            updated_by: value.updated_by,
        }
    }
}
