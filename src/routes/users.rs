use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    routing::{delete, get, patch, post},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter, QuerySelect, TryIntoModel,
};
use validator::Validate;

use crate::{
    entities_helper::{UserActiveModel, UserColumn, UserEntity, UserModel},
    serializers::{
        pagination::{Pagination, PaginationMeta},
        users::{CreateUserSerializer, ReadUserSerializer, UpdateUserSerializer},
    },
    set_fields,
    state::AppState,
    utils::{
        extractors::auth::AuthUser,
        password::create_password,
        response::{CustomResponse, to_error_response, to_error_response_with_message},
    },
};

async fn create_users(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let hashed_password: String = create_password(&payload.password)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let active_model: UserActiveModel = UserActiveModel {
        name: Set(payload.name),
        email: Set(payload.email),
        password: Set(hashed_password),
        phone: Set(payload.phone),
        ..Default::default()
    };

    // save user
    let active_model: UserActiveModel = active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // convert to ReadUserSerializer serializer
    let serializer: ReadUserSerializer = active_model
        .try_into_model()
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .into();
    Ok(
        CustomResponse::<ReadUserSerializer, ()>::builder(serializer)
            .message("User created successfully.")
            .status_code(StatusCode::CREATED)
            .build(),
    )
}

async fn list_users(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<Response<Body>, Response<Body>> {
    // Query a single record
    let page_size: u64 = pagination.limit();
    let instances: Vec<UserModel> = UserEntity::find()
        .limit(page_size)
        .offset(pagination.offset())
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    // Convert model to serializer
    let serializers: Vec<ReadUserSerializer> =
        instances.into_iter().map(|model| model.into()).collect();

    let total: u64 = UserEntity::find().count(&state.database).await.unwrap();
    let page: u64 = pagination.page();
    let total_pages: u64 = (total + page_size - 1) / page_size;
    let pagination_meta: PaginationMeta =
        PaginationMeta::build(total, total_pages, page_size, page);
    Ok(
        CustomResponse::<Vec<ReadUserSerializer>, PaginationMeta>::builder(serializers)
            .meta(pagination_meta)
            .build(),
    )
}

async fn get_user(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    // Query a single record
    let instance = UserEntity::find()
        .filter(UserColumn::UserId.eq(user_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("User not found.", StatusCode::NOT_FOUND))?;

    // Convert model to serializer
    let serializer: ReadUserSerializer = instance.into();
    Ok(CustomResponse::<ReadUserSerializer, ()>::builder(serializer).build())
}

async fn update_user(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
    Path(user_id): Path<i64>,
    Json(payload): Json<UpdateUserSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;
    // Find existing Retreat
    let instance: UserModel = UserEntity::find()
        .filter(UserColumn::UserId.eq(user_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("User not found.", StatusCode::NOT_FOUND))?;

    // Convert to ActiveModel for editing
    let mut active_model: UserActiveModel = instance.into_active_model();

    set_fields!(active_model, payload, name, email, phone);

    // Save the updated Retreat
    let instance = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert to serializer
    let serializer: ReadUserSerializer = instance.into();

    // Return success
    Ok(
        CustomResponse::<ReadUserSerializer, ()>::builder(serializer)
            .message("User updated successfully.")
            .status_code(StatusCode::OK)
            .build(),
    )
}

async fn delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    // Query a single record
    let instance = UserEntity::find()
        .filter(UserColumn::UserId.eq(user_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("User not found.", StatusCode::NOT_FOUND))?;

    // Convert to ActiveModel for editing
    let active_model: UserActiveModel = instance.into_active_model();

    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert model to serializer
    Ok(CustomResponse::<(), ()>::builder({})
        .message("User deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

pub fn users_router() -> Router<AppState> {
    let router = Router::new()
        .route("/users/", post(create_users))
        .route("/users/", get(list_users))
        .route("/users/{user_id}/", get(get_user))
        .route("/users/{user_id}/", patch(update_user))
        .route("/users/{user_id}/", delete(delete_user));
    return router;
}
