use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Retreats::Table)
                    .add_column(ColumnDef::new(Retreats::ThumbnailImage).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Retreats::Table)
                    .add_column(ColumnDef::new(Retreats::BannerImage).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Categories::Table)
                    .add_column(ColumnDef::new(Categories::ThumbnailImage).text().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Retreats::Table)
                    .drop_column(Retreats::ThumbnailImage)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Retreats::Table)
                    .drop_column(Retreats::BannerImage)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Categories::Table)
                    .drop_column(Categories::ThumbnailImage)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Retreats {
    Table,
    ThumbnailImage,
    BannerImage,
}

#[derive(DeriveIden)]
enum Categories {
    Table,
    ThumbnailImage,
}
