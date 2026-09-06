use crate::entities_helper::NewsletterSubscriberModel;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateNewsletterSubscriptionSerializer {
    #[validate(email)]
    pub email: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ReadNewsletterSubscriptionSerializer {
    pub id: i64,
    pub email: String,
    pub user_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<NewsletterSubscriberModel> for ReadNewsletterSubscriptionSerializer {
    fn from(value: NewsletterSubscriberModel) -> Self {
        ReadNewsletterSubscriptionSerializer {
            id: value.id,
            email: value.email,
            user_id: value.user_id,
            created_at: value.created_at.to_string(),
            updated_at: value.updated_at.to_string(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct NewsletterSubscriptionStatusSerializer {
    pub subscribed: bool,
    pub email: Option<String>,
    pub user_id: Option<i64>,
}