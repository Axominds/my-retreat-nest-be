use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"ALTER TABLE "retreats" ADD COLUMN "story" TEXT NULL;"#,
        )
        .await?;
        // Seed the new long-form story from the existing description so no
        // tenant story goes blank; the two columns diverge from here on.
        db.execute_unprepared(
            r#"UPDATE "retreats" SET "story" = "description" WHERE "description" IS NOT NULL;"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"ALTER TABLE "retreats" DROP COLUMN "story";"#,
        )
        .await?;
        Ok(())
    }
}
