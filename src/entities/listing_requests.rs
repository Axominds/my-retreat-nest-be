use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "listing_requests")]
pub struct Model {
    #[sea_orm(primary_key)]
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
    pub social_links: Json,
    pub status: String,
    pub reviewed_by: Option<i64>,
    pub reviewed_at: Option<DateTimeWithTimeZone>,
    pub rejection_reason: Option<String>,
    pub retreat_id: Option<i64>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::categories::Entity",
        from = "Column::CategoryId",
        to = "super::categories::Column::CategoryId",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Categories,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::ReviewedBy",
        to = "super::users::Column::UserId",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Users,
    #[sea_orm(
        belongs_to = "super::retreats::Entity",
        from = "Column::RetreatId",
        to = "super::retreats::Column::RetreatId",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Retreats,
}

impl ActiveModelBehavior for ActiveModel {}
