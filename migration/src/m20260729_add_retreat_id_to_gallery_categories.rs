use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(GalleryCategories::Table)
                    .add_column(
                        ColumnDef::new(GalleryCategories::RetreatId)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(GalleryCategories::Table, GalleryCategories::RetreatId)
                    .to(Retreats::Table, Retreats::RetreatId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
            CREATE INDEX IF NOT EXISTS idx_gallery_categories_retreat_id
            ON "gallery_categories" ("retreat_id");
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("gallery_categories_retreat_id_fkey")
                    .table(GalleryCategories::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(GalleryCategories::Table)
                    .drop_column(GalleryCategories::RetreatId)
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();

        db.execute_unprepared(
            r#"DROP INDEX IF EXISTS idx_gallery_categories_retreat_id;"#,
        )
        .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum GalleryCategories {
    Table,
    RetreatId,
}

#[derive(DeriveIden)]
enum Retreats {
    Table,
    RetreatId,
}
