use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(r#"UPDATE "retreats" SET "email" = '' WHERE "email" IS NULL;"#)
            .await?;
        db.execute_unprepared(r#"UPDATE "retreats" SET "phone" = '' WHERE "phone" IS NULL;"#)
            .await?;
        db.execute_unprepared(r#"UPDATE "retreats" SET "latitude" = 0 WHERE "latitude" IS NULL;"#)
            .await?;
        db.execute_unprepared(
            r#"UPDATE "retreats" SET "longitude" = 0 WHERE "longitude" IS NULL;"#,
        )
        .await?;

        db.execute_unprepared(r#"ALTER TABLE "retreats" ALTER COLUMN "email" SET NOT NULL;"#)
            .await?;
        db.execute_unprepared(r#"ALTER TABLE "retreats" ALTER COLUMN "phone" SET NOT NULL;"#)
            .await?;
        db.execute_unprepared(
            r#"ALTER TABLE "retreats" ALTER COLUMN "latitude" SET NOT NULL;"#,
        )
        .await?;
        db.execute_unprepared(
            r#"ALTER TABLE "retreats" ALTER COLUMN "longitude" SET NOT NULL;"#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"ALTER TABLE "retreats" ALTER COLUMN "email" DROP NOT NULL;"#,
        )
        .await?;
        db.execute_unprepared(
            r#"ALTER TABLE "retreats" ALTER COLUMN "phone" DROP NOT NULL;"#,
        )
        .await?;
        db.execute_unprepared(
            r#"ALTER TABLE "retreats" ALTER COLUMN "latitude" DROP NOT NULL;"#,
        )
        .await?;
        db.execute_unprepared(
            r#"ALTER TABLE "retreats" ALTER COLUMN "longitude" DROP NOT NULL;"#,
        )
        .await?;

        Ok(())
    }
}
