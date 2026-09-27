use sea_orm::prelude::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{entities_helper::RetreatPackagesModel, utils::serializer::deserialize_some};

#[derive(Deserialize, Clone, Debug, Validate)]
pub struct CreatePackageSerializer {
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub description: Option<String>,
    /// Optional room type this package is sold against. Must belong to the
    /// same retreat; verified by the route.
    pub room_type_id: Option<i64>,
    pub duration_nights: Option<i32>,
    /// Comma separated list of what the package includes.
    pub includes: Option<String>,
    pub price: Option<Decimal>,
    #[serde(default)]
    pub is_featured: bool,
    #[serde(default)]
    pub display_order: i32,
}

#[derive(Deserialize, Clone, Debug, Validate)]
pub struct UpdatePackageSerializer {
    pub name: Option<String>,
    // `deserialize_some` turns an explicit JSON `null` into `Some(None)`, which
    // is what makes clearing a field possible. Plain `Option<Option<T>>` would
    // collapse `null` and "field absent" into the same `None`, so a client could
    // never unset a value.
    #[serde(default, deserialize_with = "deserialize_some")]
    pub description: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub room_type_id: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub duration_nights: Option<Option<i32>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub includes: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub price: Option<Option<Decimal>>,
    pub is_featured: Option<bool>,
    pub display_order: Option<i32>,
}

#[derive(Serialize, Clone, Debug, Validate)]
pub struct ReadPackageSerializer {
    pub retreat_package_id: i64,
    pub retreat_id: i64,
    pub room_type_id: Option<i64>,
    /// Resolved from `room_type_id` so clients can render the package without a
    /// second lookup. `None` once the linked room type is deleted.
    pub room_type_name: Option<String>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub duration_nights: Option<i32>,
    pub includes: Option<String>,
    pub price: Option<Decimal>,
    pub is_featured: bool,
    pub display_order: i32,
}

impl From<RetreatPackagesModel> for ReadPackageSerializer {
    fn from(value: RetreatPackagesModel) -> Self {
        ReadPackageSerializer {
            retreat_package_id: value.retreat_package_id,
            retreat_id: value.retreat_id,
            room_type_id: value.room_type_id,
            // Resolved by the route via `with_room_type_name`.
            room_type_name: None,
            name: value.name,
            slug: value.slug,
            description: value.description,
            duration_nights: value.duration_nights,
            includes: value.includes,
            price: value.price,
            is_featured: value.is_featured,
            display_order: value.display_order,
        }
    }
}

impl ReadPackageSerializer {
    /// Attaches the resolved room type name once the room type has been loaded.
    pub fn with_room_type_name(mut self, room_type_name: Option<String>) -> Self {
        self.room_type_name = room_type_name;
        self
    }
}
