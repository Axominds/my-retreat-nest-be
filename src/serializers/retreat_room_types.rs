use sea_orm::prelude::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{entities_helper::RetreatRoomTypesModel, map_fields, utils::serializer::deserialize_some};

#[derive(Deserialize, Clone, Debug, Validate)]
pub struct CreateRoomTypeSerializer {
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub description: Option<String>,
    pub size_sqm: Option<i32>,
    pub max_guests: Option<i32>,
    pub bed_configuration: Option<String>,
    /// Comma separated list of in-room amenities.
    pub amenities: Option<String>,
    pub price_per_night: Option<Decimal>,
    #[serde(default)]
    pub is_featured: bool,
    #[serde(default)]
    pub display_order: i32,
}

#[derive(Deserialize, Clone, Debug, Validate)]
pub struct UpdateRoomTypeSerializer {
    pub name: Option<String>,
    // `deserialize_some` turns an explicit JSON `null` into `Some(None)`, which
    // is what makes clearing a field possible. Plain `Option<Option<T>>` would
    // collapse `null` and "field absent" into the same `None`, so a client could
    // never unset a value.
    #[serde(default, deserialize_with = "deserialize_some")]
    pub description: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub size_sqm: Option<Option<i32>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub max_guests: Option<Option<i32>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub bed_configuration: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub amenities: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub price_per_night: Option<Option<Decimal>>,
    pub is_featured: Option<bool>,
    pub display_order: Option<i32>,
}

#[derive(Serialize, Clone, Debug, Validate)]
pub struct ReadRoomTypeSerializer {
    pub retreat_room_type_id: i64,
    pub retreat_id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub size_sqm: Option<i32>,
    pub max_guests: Option<i32>,
    pub bed_configuration: Option<String>,
    pub amenities: Option<String>,
    pub price_per_night: Option<Decimal>,
    pub is_featured: bool,
    pub image_path: Option<String>,
    pub display_order: i32,
}

impl From<RetreatRoomTypesModel> for ReadRoomTypeSerializer {
    fn from(value: RetreatRoomTypesModel) -> Self {
        map_fields!(value, ReadRoomTypeSerializer, {
            retreat_room_type_id,
            retreat_id,
            name,
            slug,
            description,
            size_sqm,
            max_guests,
            bed_configuration,
            amenities,
            price_per_night,
            is_featured,
            image_path,
            display_order,
        })
    }
}
