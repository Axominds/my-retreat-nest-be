use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        manager
            .create_table(
                Table::create()
                    .table(NewsletterSubscribers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NewsletterSubscribers::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(NewsletterSubscribers::Email).string().not_null())
                    .col(
                        ColumnDef::new(NewsletterSubscribers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(NewsletterSubscribers::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("idx_newsletter_subscribers_email")
                            .col(NewsletterSubscribers::Email),
                    )
                    .to_owned(),
            )
            .await?;

        db.execute_unprepared(
            r#"
            CREATE TRIGGER trigger_set_updated_at
            BEFORE UPDATE ON "newsletter_subscribers"
            FOR EACH ROW
            EXECUTE FUNCTION set_updated_at();
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"DROP TRIGGER IF EXISTS trigger_set_updated_at ON "newsletter_subscribers";"#,
        )
        .await?;

        manager
            .drop_table(Table::drop().table(NewsletterSubscribers::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum NewsletterSubscribers {
    Table,
    Id,
    Email,
    CreatedAt,
    UpdatedAt,
}