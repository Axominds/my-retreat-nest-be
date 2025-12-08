use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::{delete, get, patch, post},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    TryIntoModel,
};
use validator::Validate;

use crate::{
    entities_helper::{
        RetreatColumn, RetreatEntity, RetreatReviewActiveModel, RetreatReviewColumn,
        RetreatReviewEntity, RetreatReviewModel,
    },
    serializers::retreat_reviews::{
        CreateRetreatReviewSerializer, ReadRetreatReviewSerializer, UpdateRetreatReviewSerializer,
    },
    set_fields,
    state::AppState,
    utils::{
        extractors::auth::AuthUser,
        response::{CustomResponse, to_error_response, to_error_response_with_message},
    },
};

async fn create_retreat_review(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(retreat_id): Path<i64>,
    Json(payload): Json<CreateRetreatReviewSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;
    // Find existing Retreat
    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    let active_model: RetreatReviewActiveModel = RetreatReviewActiveModel {
        rating: Set(payload.rating),
        review: Set(payload.review),
        user_id: Set(user.user_id),
        retreat_id: Set(retreat_id),
        ..Default::default()
    };

    // save user
    let active_model: RetreatReviewActiveModel = active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // convert to ReadUserSerializer serializer
    let serializer: ReadRetreatReviewSerializer = active_model
        .try_into_model()
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .into();

    Ok(CustomResponse::<ReadRetreatReviewSerializer, ()>::builder(serializer).build())
}

async fn list_retreat_review(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    // Find existing Retreat
    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    let instances: Vec<RetreatReviewModel> = RetreatReviewEntity::find()
        .filter(RetreatReviewColumn::RetreatId.eq(retreat_id))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert model to serializer
    let serializers: Vec<ReadRetreatReviewSerializer> =
        instances.into_iter().map(|model| model.into()).collect();

    Ok(CustomResponse::<Vec<ReadRetreatReviewSerializer>, ()>::builder(serializers).build())
}

async fn update_retreat_review(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((retreat_id, review_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateRetreatReviewSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    let instance: RetreatReviewModel = RetreatReviewEntity::find()
        .filter(RetreatReviewColumn::ReviewId.eq(review_id))
        .filter(RetreatReviewColumn::RetreatId.eq(retreat_id))
        .filter(RetreatReviewColumn::UserId.eq(user.user_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat review not found.", StatusCode::NOT_FOUND)
        })?;

    // Convert to ActiveModel for editing
    let mut active_model: RetreatReviewActiveModel = instance.into_active_model();

    set_fields!(active_model, payload, rating, review);

    // Save the updated Retreat
    let instance = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert to serializer
    let serializer: ReadRetreatReviewSerializer = instance.into();

    Ok(CustomResponse::<ReadRetreatReviewSerializer, ()>::builder(serializer).build())
}

async fn delete_retreat_review(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((retreat_id, review_id)): Path<(i64, i64)>,
) -> Result<Response<Body>, Response<Body>> {
    let instance: RetreatReviewModel = RetreatReviewEntity::find()
        .filter(RetreatReviewColumn::ReviewId.eq(review_id))
        .filter(RetreatReviewColumn::RetreatId.eq(retreat_id))
        .filter(RetreatReviewColumn::UserId.eq(user.user_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat review not found.", StatusCode::NOT_FOUND)
        })?;

    // Convert to ActiveModel for editing
    let active_model: RetreatReviewActiveModel = instance.into_active_model();

    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert model to serializer
    Ok(CustomResponse::<(), ()>::builder({})
        .message("Retreat review deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

pub fn retreat_review_router() -> Router<AppState> {
    let router = Router::new()
        .route(
            "/retreats/{retreat_id}/reviews/",
            post(create_retreat_review),
        )
        .route("/retreats/{retreat_id}/reviews/", get(list_retreat_review))
        .route(
            "/retreats/{retreat_id}/reviews/{review_id}/",
            patch(update_retreat_review),
        )
        .route(
            "/retreats/{retreat_id}/reviews/{review_id}/",
            delete(delete_retreat_review),
        );
    return router;
}
