use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "amenities")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub amenity_id: i64,
    #[sea_orm(unique)]
    pub label: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::retreat_amenities::Entity")]
    RetreatAmenities,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::CreatedBy",
        to = "super::users::Column::UserId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Users2,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UpdatedBy",
        to = "super::users::Column::UserId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Users1,
}

impl Related<super::retreat_amenities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RetreatAmenities.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
