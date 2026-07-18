use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"ALTER TABLE "retreat_galleries" DROP CONSTRAINT IF EXISTS "retreat_galleries_gallery_category_id_fkey";"#,
        )
        .await?;

        db.execute_unprepared(
            r#"ALTER TABLE "retreat_galleries" ALTER COLUMN "gallery_category_id" SET NOT NULL;"#,
        )
        .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(
                        RetreatGalleries::Table,
                        RetreatGalleries::GalleryCategoryId,
                    )
                    .to(
                        GalleryCategories::Table,
                        GalleryCategories::GalleryCategoryId,
                    )
                    .on_delete(ForeignKeyAction::Restrict)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"ALTER TABLE "retreat_galleries" DROP CONSTRAINT IF EXISTS "retreat_galleries_gallery_category_id_fkey";"#,
        )
        .await?;

        db.execute_unprepared(
            r#"ALTER TABLE "retreat_galleries" ALTER COLUMN "gallery_category_id" DROP NOT NULL;"#,
        )
        .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(
                        RetreatGalleries::Table,
                        RetreatGalleries::GalleryCategoryId,
                    )
                    .to(
                        GalleryCategories::Table,
                        GalleryCategories::GalleryCategoryId,
                    )
                    .on_delete(ForeignKeyAction::SetNull)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum RetreatGalleries {
    Table,
    GalleryCategoryId,
}

#[derive(DeriveIden)]
enum GalleryCategories {
    Table,
    GalleryCategoryId,
}
