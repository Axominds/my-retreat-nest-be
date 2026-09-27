use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Create retreat_room_types table
        manager
            .create_table(
                Table::create()
                    .table(RetreatRoomTypes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RetreatRoomTypes::RetreatRoomTypeId)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RetreatRoomTypes::RetreatId).big_integer().not_null())
                    .col(ColumnDef::new(RetreatRoomTypes::Name).string().not_null())
                    .col(ColumnDef::new(RetreatRoomTypes::Slug).string().not_null())
                    .col(ColumnDef::new(RetreatRoomTypes::Description).text().null())
                    .col(ColumnDef::new(RetreatRoomTypes::SizeSqm).integer().null())
                    .col(ColumnDef::new(RetreatRoomTypes::MaxGuests).integer().null())
                    .col(ColumnDef::new(RetreatRoomTypes::BedConfiguration).string().null())
                    // Comma separated list, matching the `overview_details` convention.
                    .col(ColumnDef::new(RetreatRoomTypes::Amenities).text().null())
                    .col(ColumnDef::new(RetreatRoomTypes::PricePerNight).decimal().null())
                    .col(
                        ColumnDef::new(RetreatRoomTypes::IsFeatured)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(RetreatRoomTypes::ImagePath).string().null())
                    .col(
                        ColumnDef::new(RetreatRoomTypes::DisplayOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(RetreatRoomTypes::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(RetreatRoomTypes::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(RetreatRoomTypes::CreatedBy).big_integer().null())
                    .col(ColumnDef::new(RetreatRoomTypes::UpdatedBy).big_integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(RetreatRoomTypes::Table, RetreatRoomTypes::RetreatId)
                            .to(Retreats::Table, Retreats::RetreatId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(RetreatRoomTypes::Table, RetreatRoomTypes::CreatedBy)
                            .to(Users::Table, Users::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(RetreatRoomTypes::Table, RetreatRoomTypes::UpdatedBy)
                            .to(Users::Table, Users::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Slugs only need to be unique within a retreat, not globally.
        manager
            .create_index(
                Index::create()
                    .name("idx_retreat_room_types_retreat_slug_unique")
                    .table(RetreatRoomTypes::Table)
                    .col(RetreatRoomTypes::RetreatId)
                    .col(RetreatRoomTypes::Slug)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_retreat_room_types_retreat_id")
                    .table(RetreatRoomTypes::Table)
                    .col(RetreatRoomTypes::RetreatId)
                    .to_owned(),
            )
            .await?;

        // Create trigger for updated_at on retreat_room_types
        db.execute_unprepared(
            r#"
            CREATE TRIGGER trigger_set_updated_at_retreat_room_types
            BEFORE UPDATE ON "retreat_room_types"
            FOR EACH ROW
            EXECUTE FUNCTION set_updated_at();
            "#,
        )
        .await?;

        // Create retreat_packages table
        manager
            .create_table(
                Table::create()
                    .table(RetreatPackages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RetreatPackages::RetreatPackageId)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RetreatPackages::RetreatId).big_integer().not_null())
                    // Nullable so a package survives its room type being deleted; the
                    // package simply stops advertising a specific room.
                    .col(ColumnDef::new(RetreatPackages::RoomTypeId).big_integer().null())
                    .col(ColumnDef::new(RetreatPackages::Name).string().not_null())
                    .col(ColumnDef::new(RetreatPackages::Slug).string().not_null())
                    .col(ColumnDef::new(RetreatPackages::Description).text().null())
                    .col(ColumnDef::new(RetreatPackages::DurationNights).integer().null())
                    // Comma separated list, matching the `overview_details` convention.
                    .col(ColumnDef::new(RetreatPackages::Includes).text().null())
                    .col(ColumnDef::new(RetreatPackages::Price).decimal().null())
                    .col(
                        ColumnDef::new(RetreatPackages::IsFeatured)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(RetreatPackages::DisplayOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(RetreatPackages::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(RetreatPackages::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(RetreatPackages::CreatedBy).big_integer().null())
                    .col(ColumnDef::new(RetreatPackages::UpdatedBy).big_integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(RetreatPackages::Table, RetreatPackages::RetreatId)
                            .to(Retreats::Table, Retreats::RetreatId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(RetreatPackages::Table, RetreatPackages::RoomTypeId)
                            .to(RetreatRoomTypes::Table, RetreatRoomTypes::RetreatRoomTypeId)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(RetreatPackages::Table, RetreatPackages::CreatedBy)
                            .to(Users::Table, Users::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(RetreatPackages::Table, RetreatPackages::UpdatedBy)
                            .to(Users::Table, Users::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_retreat_packages_retreat_slug_unique")
                    .table(RetreatPackages::Table)
                    .col(RetreatPackages::RetreatId)
                    .col(RetreatPackages::Slug)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_retreat_packages_retreat_id")
                    .table(RetreatPackages::Table)
                    .col(RetreatPackages::RetreatId)
                    .to_owned(),
            )
            .await?;

        // Create trigger for updated_at on retreat_packages
        db.execute_unprepared(
            r#"
            CREATE TRIGGER trigger_set_updated_at_retreat_packages
            BEFORE UPDATE ON "retreat_packages"
            FOR EACH ROW
            EXECUTE FUNCTION set_updated_at();
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // retreat_packages references retreat_room_types, so drop it first.
        manager
            .drop_table(Table::drop().table(RetreatPackages::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(RetreatRoomTypes::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum RetreatRoomTypes {
    Table,
    RetreatRoomTypeId,
    RetreatId,
    Name,
    Slug,
    Description,
    SizeSqm,
    MaxGuests,
    BedConfiguration,
    Amenities,
    PricePerNight,
    IsFeatured,
    ImagePath,
    DisplayOrder,
    CreatedAt,
    UpdatedAt,
    CreatedBy,
    UpdatedBy,
}

#[derive(DeriveIden)]
enum RetreatPackages {
    Table,
    RetreatPackageId,
    RetreatId,
    RoomTypeId,
    Name,
    Slug,
    Description,
    DurationNights,
    Includes,
    Price,
    IsFeatured,
    DisplayOrder,
    CreatedAt,
    UpdatedAt,
    CreatedBy,
    UpdatedBy,
}

#[derive(DeriveIden)]
enum Retreats {
    Table,
    RetreatId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    UserId,
}
