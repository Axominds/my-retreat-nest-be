use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"ALTER TABLE "retreat_users" DROP COLUMN "is_owner";"#,
        )
        .await?;

        manager
            .create_table(
                Table::create()
                    .table(ListingRequests::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ListingRequests::ListingRequestId)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ListingRequests::OwnerName).string().not_null())
                    .col(ColumnDef::new(ListingRequests::OwnerEmail).string().not_null())
                    .col(ColumnDef::new(ListingRequests::OwnerPhone).string().null())
                    .col(ColumnDef::new(ListingRequests::RetreatName).string().not_null())
                    .col(ColumnDef::new(ListingRequests::RetreatDescription).text().null())
                    .col(ColumnDef::new(ListingRequests::CategoryId).big_integer().not_null())
                    .col(ColumnDef::new(ListingRequests::RetreatEmail).string().null())
                    .col(ColumnDef::new(ListingRequests::RetreatPhone).string().null())
                    .col(ColumnDef::new(ListingRequests::Latitude).decimal().null())
                    .col(ColumnDef::new(ListingRequests::Longitude).decimal().null())
                    .col(ColumnDef::new(ListingRequests::Address).text().null())
                    .col(ColumnDef::new(ListingRequests::BudgetMin).decimal().null())
                    .col(ColumnDef::new(ListingRequests::BudgetMax).decimal().null())
                    .col(ColumnDef::new(ListingRequests::SocialLinks).json().not_null().default(Expr::value("{}")))
                    .col(
                        ColumnDef::new(ListingRequests::Status)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(ListingRequests::ReviewedBy).big_integer().null())
                    .col(ColumnDef::new(ListingRequests::ReviewedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(ListingRequests::RejectionReason).text().null())
                    .col(ColumnDef::new(ListingRequests::RetreatId).big_integer().null())
                    .col(
                        ColumnDef::new(ListingRequests::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ListingRequests::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ListingRequests::Table, ListingRequests::CategoryId)
                            .to(Categories::Table, Categories::CategoryId)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ListingRequests::Table, ListingRequests::ReviewedBy)
                            .to(Users::Table, Users::UserId)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ListingRequests::Table, ListingRequests::RetreatId)
                            .to(Retreats::Table, Retreats::RetreatId)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        db.execute_unprepared(
            r#"
            CREATE TRIGGER trigger_set_updated_at
            BEFORE UPDATE ON "listing_requests"
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
            r#"DROP TRIGGER IF EXISTS trigger_set_updated_at ON "listing_requests";"#,
        )
        .await?;

        manager
            .drop_table(Table::drop().table(ListingRequests::Table).to_owned())
            .await?;

        db.execute_unprepared(
            r#"ALTER TABLE "retreat_users" ADD COLUMN "is_owner" boolean NOT NULL DEFAULT false;"#,
        )
        .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum ListingRequests {
    Table,
    ListingRequestId,
    OwnerName,
    OwnerEmail,
    OwnerPhone,
    RetreatName,
    RetreatDescription,
    CategoryId,
    RetreatEmail,
    RetreatPhone,
    Latitude,
    Longitude,
    Address,
    BudgetMin,
    BudgetMax,
    SocialLinks,
    Status,
    ReviewedBy,
    ReviewedAt,
    RejectionReason,
    RetreatId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Categories {
    Table,
    CategoryId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    UserId,
}

#[derive(DeriveIden)]
enum Retreats {
    Table,
    RetreatId,
}
