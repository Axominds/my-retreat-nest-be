pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_users;
mod m20250914_055232_updated_at_trigger;
mod m20250914_055447_add_updated_at_trigger_in_users;
mod m20251008_141025_retreat_and_retreat_users;
mod m20251030_145305_user_wishlist;
mod m20251030_150713_retreat_reviews;
mod m20251103_162943_retreat_gallery;
mod m20251109_154739_gallery_category;
mod m20260727_password_reset_tokens;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_users::Migration),
            Box::new(m20250914_055232_updated_at_trigger::Migration),
            Box::new(m20250914_055447_add_updated_at_trigger_in_users::Migration),
            Box::new(m20251008_141025_retreat_and_retreat_users::Migration),
            Box::new(m20251030_145305_user_wishlist::Migration),
            Box::new(m20251030_150713_retreat_reviews::Migration),
            Box::new(m20251103_162943_retreat_gallery::Migration),
            Box::new(m20251109_154739_gallery_category::Migration),
            Box::new(m20260727_password_reset_tokens::Migration),
        ]
    }
}
