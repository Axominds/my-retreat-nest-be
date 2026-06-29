use sea_orm::Database;

use crate::{env, utils::mail::Mailer};


#[derive(Clone, Debug)]
pub struct AppState {
    pub database: sea_orm::DatabaseConnection,
    pub mailer: Mailer,
}

impl AppState {
    pub async fn new() -> Self {
        Self {
            database: Database::connect(&env::ENV.database_url).await.unwrap(),
            mailer: Mailer::new(),
        }
    }
}
