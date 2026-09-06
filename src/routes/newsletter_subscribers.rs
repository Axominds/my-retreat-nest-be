use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{Response, StatusCode},
    routing::{delete, get, post},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbErr, EntityTrait, ExprTrait,
    IntoActiveModel, QueryFilter, TryIntoModel,
};
use validator::Validate;

use crate::{
    entities_helper::{
        NewsletterSubscriberActiveModel, NewsletterSubscriberColumn, NewsletterSubscriberEntity,
        NewsletterSubscriberModel, UserModel,
    },
    serializers::newsletter_subscribers::{
        CreateNewsletterSubscriptionSerializer, NewsletterSubscriptionStatusSerializer,
        ReadNewsletterSubscriptionSerializer,
    },
    state::AppState,
    utils::{
        extractors::auth::{AuthUser, OptionalAuthUser},
        response::{CustomResponse, to_error_response},
    },
};

async fn find_subscription_for_user(
    db: &sea_orm::DatabaseConnection,
    user: &UserModel,
) -> Result<Option<NewsletterSubscriberModel>, Response<Body>> {
    NewsletterSubscriberEntity::find()
        .filter(
            NewsletterSubscriberColumn::UserId
                .eq(user.user_id)
                .or(NewsletterSubscriberColumn::Email.eq(&user.email)),
        )
        .one(db)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))
}

async fn subscribe_to_newsletter(
    State(state): State<AppState>,
    OptionalAuthUser(auth_user): OptionalAuthUser,
    Json(payload): Json<CreateNewsletterSubscriptionSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let (email, user_id) = match &auth_user {
        Some(user) => (user.email.clone(), Some(user.user_id)),
        None => (payload.email, None),
    };

    let existing = match user_id {
        Some(uid) => NewsletterSubscriberEntity::find()
            .filter(
                NewsletterSubscriberColumn::Email
                    .eq(&email)
                    .or(NewsletterSubscriberColumn::UserId.eq(uid)),
            )
            .one(&state.database)
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?,
        None => NewsletterSubscriberEntity::find()
            .filter(NewsletterSubscriberColumn::Email.eq(&email))
            .one(&state.database)
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?,
    };

    if let Some(mut model) = existing {
        if user_id.is_some() && model.user_id.is_none() {
            let mut active_model = model.clone().into_active_model();
            active_model.user_id = Set(user_id);
            model = active_model
                .update(&state.database)
                .await
                .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
                .try_into_model()
                .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
        }
        let serializer: ReadNewsletterSubscriptionSerializer = model.into();
        return Ok(
            CustomResponse::<ReadNewsletterSubscriptionSerializer, ()>::builder(serializer)
                .message("You're already subscribed to the newsletter.")
                .status_code(StatusCode::OK)
                .build(),
        );
    }

    let active_model: NewsletterSubscriberActiveModel = NewsletterSubscriberActiveModel {
        email: Set(email),
        user_id: Set(user_id),
        ..Default::default()
    };

    let saved = match active_model.save(&state.database).await {
        Ok(saved) => saved,
        Err(e) => {
            if is_unique_violation(&e) {
                return Ok(
                    CustomResponse::<(), ()>::builder({})
                        .message("You're already subscribed to the newsletter.")
                        .status_code(StatusCode::OK)
                        .build(),
                );
            }
            return Err(to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR));
        }
    };

    let model: NewsletterSubscriberModel = saved
        .try_into_model()
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadNewsletterSubscriptionSerializer = model.into();

    Ok(CustomResponse::<ReadNewsletterSubscriptionSerializer, ()>::builder(
        serializer,
    )
    .message("You've subscribed to the newsletter. Welcome aboard!")
    .status_code(StatusCode::CREATED)
    .build())
}

async fn get_subscription_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<Response<Body>, Response<Body>> {
    let existing = find_subscription_for_user(&state.database, &user).await?;

    let status = NewsletterSubscriptionStatusSerializer {
        subscribed: existing.is_some(),
        email: existing.as_ref().map(|m| m.email.clone()).or(Some(user.email.clone())),
        user_id: Some(user.user_id),
    };

    Ok(CustomResponse::<NewsletterSubscriptionStatusSerializer, ()>::builder(
        status,
    )
    .build())
}

async fn unsubscribe_from_newsletter(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<Response<Body>, Response<Body>> {
    let existing = find_subscription_for_user(&state.database, &user).await?;

    match existing {
        Some(model) => {
            model
                .into_active_model()
                .delete(&state.database)
                .await
                .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

            Ok(CustomResponse::<(), ()>::builder({})
                .message("You've unsubscribed from the newsletter.")
                .status_code(StatusCode::OK)
                .build())
        }
        None => Ok(CustomResponse::<(), ()>::builder({})
            .message("You're not subscribed to the newsletter.")
            .status_code(StatusCode::OK)
            .build()),
    }
}

fn is_unique_violation(e: &DbErr) -> bool {
    let msg = e.to_string().to_lowercase();
    msg.contains("unique") || msg.contains("duplicate") || msg.contains("23505")
}

pub fn newsletter_subscriber_router() -> Router<AppState> {
    Router::new()
        .route(
            "/newsletter-subscribers/subscribe/",
            post(subscribe_to_newsletter),
        )
        .route(
            "/newsletter-subscribers/status/",
            get(get_subscription_status),
        )
        .route(
            "/newsletter-subscribers/unsubscribe/",
            delete(unsubscribe_from_newsletter),
        )
}