use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{Response, StatusCode},
    routing::{delete, get, patch, post},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, QuerySelect, TryIntoModel
};

use validator::Validate;

use crate::{
    entities_helper::{
        RetreatActiveModel, RetreatColumn, RetreatEntity, RetreatModel, RetreatUserActiveModel,
        RetreatUserColumn, RetreatUserEntity, RetreatUserModel, UserActiveModel, UserColumn,
        UserEntity, UserModel,
    },
    serializers::{pagination::{Pagination, PaginationMeta}, retreats::{
        CreateRetreatSerializer, CreateRetreatUserSerializer, ReadRetreatSerializer,
        UpdateRetreatSerializer, UpdateRetreatUserSerializer,
    }},
    set_active_model_fields, set_fields,
    state::AppState,
    utils::{
        password::create_password,
        response::{CustomResponse, to_error_response, to_error_response_with_message},
    },
};

async fn create_retreat(
    State(state): State<AppState>,
    Json(payload): Json<CreateRetreatSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let active_model: RetreatActiveModel = set_active_model_fields!(payload, RetreatActiveModel, {
        name,
        description,
        category_id,
        slug,
        social_links,
        email,
        phone,
        latitude,
        longitude,
        address
    });

    // save Retreat
    let active_model: RetreatActiveModel = active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    // convert to ReadRetreatSerializer serializer
    let serializer: ReadRetreatSerializer = active_model
        .try_into_model()
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .into();
    Ok(CustomResponse::<ReadRetreatSerializer, ()>::builder(serializer)
        .message("Retreat created successfully.")
        .status_code(StatusCode::CREATED)
        .build())
}

async fn list_retreats(State(state): State<AppState>, Query(pagination): Query<Pagination>) -> Result<Response<Body>, Response<Body>> {
    // Query a single record
    let per_page: u64 = pagination.limit();
    let page: u64 = pagination.page();
    let instances: Vec<RetreatModel> = RetreatEntity::find()
        .limit(pagination.limit())
        .offset(pagination.offset())
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    // Convert model to serializer
    let serializers: Vec<ReadRetreatSerializer> =
        instances.into_iter().map(|model| model.into()).collect();
    
    let total: u64 = RetreatEntity::find().count(&state.database).await.unwrap();
    let total_pages = (total + per_page - 1) / per_page;
    let pagination_meta = PaginationMeta::build(total, total_pages, per_page, page);
    Ok(CustomResponse::<Vec<ReadRetreatSerializer>, PaginationMeta>::builder(serializers).meta(pagination_meta).build())
}

async fn get_retreat(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    // Query a single record
    let instance = RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    // Convert model to serializer
    let serializer: ReadRetreatSerializer = instance.into();
    Ok(CustomResponse::<ReadRetreatSerializer, ()>::builder(serializer).build())
}

async fn update_retreat(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
    Json(payload): Json<UpdateRetreatSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;
    // Find existing Retreat
    let instance = RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    // Convert to ActiveModel for editing
    let mut active_model: RetreatActiveModel = instance.into_active_model();

    set_fields!(
        active_model,
        payload,
        name,
        description,
        category_id,
        slug,
        social_links,
        email,
        phone,
        longitude,
        latitude,
        address,
        budget_min,
        budget_max,
        is_published
    );

    // Save the updated Retreat
    let instance = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert to serializer
    let serializer: ReadRetreatSerializer = instance.into();

    // Return success
    Ok(CustomResponse::<ReadRetreatSerializer, ()>::builder(serializer)
        .message("Retreat updated successfully.")
        .status_code(StatusCode::OK)
        .build())
}

async fn delete_retreat(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    // Query a single record
    let instance = RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    // Convert to ActiveModel for editing
    let active_model: RetreatActiveModel = instance.into_active_model();

    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert model to serializer
    Ok(CustomResponse::<(), ()>::builder({})
        .message("Retreat deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

async fn create_retreat_user(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
    Json(payload): Json<CreateRetreatUserSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    // Ensure retreat exists
    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    // Check if user exists
    let user = UserEntity::find()
        .filter(UserColumn::Email.eq(&payload.email))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let user_id: i64 = if let Some(user) = user {
        if user.name != payload.name {
            // Early return: user exists with different name
            return Ok(CustomResponse::<(), ()>::builder({})
                .message(&format!(
                    "User exists with a different name <strong>{}</strong>.",
                    user.name
                ))
                .status_code(StatusCode::ACCEPTED)
                .build());
        }
        user.user_id
    } else {
        // Create new user
        let hashed_password = create_password("tempPassword")
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

        let user_active_model = UserActiveModel {
            name: Set(payload.name),
            email: Set(payload.email),
            password: Set(hashed_password),
            ..Default::default()
        };

        let saved_user: UserModel = user_active_model
            .save(&state.database)
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
            .try_into_model()
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

        saved_user.user_id
    };

    // Associate user with retreat
    let active_model: RetreatUserActiveModel = RetreatUserActiveModel {
        retreat_id: Set(retreat_id),
        user_id: Set(user_id),
        role: Set(Some(payload.role)),
        ..Default::default()
    };

    active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(CustomResponse::<(), ()>::builder({})
        .message("Staff added successfully.")
        .status_code(StatusCode::CREATED)
        .build())
}

async fn update_retreat_user(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
    Path(retreat_user_id): Path<i64>,
    Json(payload): Json<UpdateRetreatUserSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;
    // Ensure retreat exists
    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    // Ensure retreat exists
    let instance: RetreatUserModel = RetreatUserEntity::find()
        .filter(RetreatUserColumn::RetreatUserId.eq(retreat_user_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("Staff not found.", StatusCode::NOT_FOUND))?;

    // Convert to ActiveModel for editing
    let mut active_model: RetreatUserActiveModel = instance.into_active_model();

    set_fields!(active_model, payload, role);

    Ok(CustomResponse::<(), ()>::builder({})
        .message("Staff added successfully.")
        .status_code(StatusCode::CREATED)
        .build())
}

async fn delete_retreat_user(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
    Path(retreat_user_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    // Ensure retreat exists
    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    // Ensure retreat exists
    let instance: RetreatUserModel = RetreatUserEntity::find()
        .filter(RetreatUserColumn::RetreatUserId.eq(retreat_user_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("Staff not found.", StatusCode::NOT_FOUND))?;

    // Convert to ActiveModel for editing
    let active_model: RetreatUserActiveModel = instance.into_active_model();

    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert model to serializer
    Ok(CustomResponse::<(), ()>::builder({})
        .message("Staff deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

pub fn retreat_router() -> Router<AppState> {
    let router = Router::new()
        .route("/retreats/", post(create_retreat))
        .route("/retreats/", get(list_retreats))
        .route("/retreats/{retreat_id}/", get(get_retreat))
        .route("/retreats/{retreat_id}/", patch(update_retreat))
        .route("/retreats/{retreat_id}/", delete(delete_retreat))
        .route("/retreats/{retreat_id}/users/", post(create_retreat_user))
        .route(
            "/retreats/{retreat_id}/users/{retreat_user_id}/",
            patch(update_retreat_user),
        )
        .route(
            "/retreats/{retreat_id}/users/{retreat_user_id}/",
            delete(delete_retreat_user),
        );
    return router;
}
