use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "retreat_amenities")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub retreat_amenities_id: i64,
    pub retreat_id: i64,
    pub amenity_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::retreats::Entity",
        from = "Column::RetreatId",
        to = "super::retreats::Column::RetreatId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Retreats,
    #[sea_orm(
        belongs_to = "super::amenities::Entity",
        from = "Column::AmenityId",
        to = "super::amenities::Column::AmenityId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Amenities,
}

impl Related<super::retreats::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Retreats.def()
    }
}

impl Related<super::amenities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Amenities.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
