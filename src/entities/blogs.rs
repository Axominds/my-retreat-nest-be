//! `SeaORM` Entity for blogs.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "blogs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub blog_id: i64,
    pub title: String,
    #[sea_orm(unique)]
    pub slug: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub excerpt: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub tags: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub cover_image: Option<String>,
    pub is_published: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
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

impl ActiveModelBehavior for ActiveModel {}
