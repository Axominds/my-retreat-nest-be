use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Create amenities table
        manager
            .create_table(
                Table::create()
                    .table(Amenities::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Amenities::AmenityId)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Amenities::Label).string().not_null().unique_key())
                    .col(
                        ColumnDef::new(Amenities::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Amenities::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Amenities::CreatedBy).big_integer().null())
                    .col(ColumnDef::new(Amenities::UpdatedBy).big_integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Amenities::Table, Amenities::CreatedBy)
                            .to(Users::Table, Users::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Amenities::Table, Amenities::UpdatedBy)
                            .to(Users::Table, Users::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create trigger for updated_at on amenities
        db.execute_unprepared(
            r#"
            CREATE TRIGGER trigger_set_updated_at
            BEFORE UPDATE ON "amenities"
            FOR EACH ROW
            EXECUTE FUNCTION set_updated_at();
            "#,
        )
        .await?;

        // Create retreat_amenities junction table
        manager
            .create_table(
                Table::create()
                    .table(RetreatAmenities::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RetreatAmenities::RetreatAmenitiesId)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RetreatAmenities::RetreatId).big_integer().not_null())
                    .col(ColumnDef::new(RetreatAmenities::AmenityId).big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(RetreatAmenities::Table, RetreatAmenities::RetreatId)
                            .to(Retreats::Table, Retreats::RetreatId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(RetreatAmenities::Table, RetreatAmenities::AmenityId)
                            .to(Amenities::Table, Amenities::AmenityId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("idx_retreat_amenities_unique")
                            .col(RetreatAmenities::RetreatId)
                            .col(RetreatAmenities::AmenityId),
                    )
                    .to_owned(),
            )
            .await?;

        // Add selected_amenities column to listing_requests
        manager
            .alter_table(
                Table::alter()
                    .table(ListingRequests::Table)
                    .add_column(
                        ColumnDef::new(ListingRequests::SelectedAmenities)
                            .json()
                            .null()
                            .default(Expr::value("[]")),
                    )
                    .to_owned(),
            )
            .await?;

        // Seed predefined amenities
        db.execute_unprepared(
            r#"
            INSERT INTO "amenities" ("label") VALUES
                ('Breakfast Included'),
                ('Swimming Pool'),
                ('24/7 WiFi'),
                ('Free Parking'),
                ('Spa'),
                ('Yoga Studio'),
                ('Gym'),
                ('Pet Friendly'),
                ('Airport Transfer'),
                ('Kitchen Access'),
                ('Laundry Service'),
                ('Air Conditioning'),
                ('Fireplace'),
                ('Lake Access'),
                ('Meditation Room');
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Remove selected_amenities from listing_requests
        manager
            .alter_table(
                Table::alter()
                    .table(ListingRequests::Table)
                    .drop_column(ListingRequests::SelectedAmenities)
                    .to_owned(),
            )
            .await?;

        // Drop retreat_amenities
        manager
            .drop_table(Table::drop().table(RetreatAmenities::Table).to_owned())
            .await?;

        // Drop amenities trigger and table
        db.execute_unprepared(
            r#"DROP TRIGGER IF EXISTS trigger_set_updated_at ON "amenities";"#,
        )
        .await?;

        manager
            .drop_table(Table::drop().table(Amenities::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Amenities {
    Table,
    AmenityId,
    Label,
    CreatedAt,
    UpdatedAt,
    CreatedBy,
    UpdatedBy,
}

#[derive(DeriveIden)]
enum RetreatAmenities {
    Table,
    RetreatAmenitiesId,
    RetreatId,
    AmenityId,
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

#[derive(DeriveIden)]
enum ListingRequests {
    Table,
    SelectedAmenities,
}
