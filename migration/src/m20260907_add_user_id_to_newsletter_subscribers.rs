use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(NewsletterSubscribers::Table)
                    .add_column(ColumnDef::new(NewsletterSubscribers::UserId).big_integer().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                sea_query::ForeignKey::create()
                    .name("fk_newsletter_subscribers_user")
                    .from(NewsletterSubscribers::Table, NewsletterSubscribers::UserId)
                    .to(Users::Table, Users::UserId)
                    .on_delete(ForeignKeyAction::SetNull)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                sea_query::ForeignKey::drop()
                    .name("fk_newsletter_subscribers_user")
                    .table(NewsletterSubscribers::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(NewsletterSubscribers::Table)
                    .drop_column(NewsletterSubscribers::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum NewsletterSubscribers {
    Table,
    UserId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    UserId,
}