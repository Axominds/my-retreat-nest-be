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
                    .table(Blogs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Blogs::BlogId)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Blogs::Title).string().not_null())
                    .col(ColumnDef::new(Blogs::Slug).string().not_null().unique_key())
                    .col(ColumnDef::new(Blogs::Excerpt).text().null())
                    .col(ColumnDef::new(Blogs::Content).text().not_null())
                    .col(ColumnDef::new(Blogs::Tags).text().null())
                    .col(ColumnDef::new(Blogs::CoverImage).text().null())
                    .col(
                        ColumnDef::new(Blogs::IsPublished)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Blogs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Blogs::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Blogs::CreatedBy).big_integer().null())
                    .col(ColumnDef::new(Blogs::UpdatedBy).big_integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Blogs::Table, Blogs::CreatedBy)
                            .to(Users::Table, Users::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Blogs::Table, Blogs::UpdatedBy)
                            .to(Users::Table, Users::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        db.execute_unprepared(
            r#"
            CREATE TRIGGER trigger_set_updated_at
            BEFORE UPDATE ON "blogs"
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
            r#"DROP TRIGGER IF EXISTS trigger_set_updated_at ON "blogs";"#,
        )
        .await?;

        manager
            .drop_table(Table::drop().table(Blogs::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Blogs {
    Table,
    BlogId,
    Title,
    Slug,
    Excerpt,
    Content,
    Tags,
    CoverImage,
    IsPublished,
    CreatedAt,
    UpdatedAt,
    CreatedBy,
    UpdatedBy,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    UserId,
}
