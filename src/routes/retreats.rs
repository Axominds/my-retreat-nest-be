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
        RetreatActiveModel, RetreatColumn, RetreatEntity, RetreatModel, RetreatReviewColumn,
        RetreatReviewEntity, RetreatUserActiveModel, RetreatUserColumn, RetreatUserEntity,
        RetreatUserModel, UserActiveModel, UserColumn, UserEntity, UserModel,
    },
    serializers::{
        pagination::{Paginate, PaginationMeta},
        retreats::{
            CreateRetreatSerializer, CreateRetreatUserSerializer, ReadRetreatSerializer,
            ReadRetreatUserSerializer, RetreatFilter, UpdateRetreatSerializer,
            UpdateRetreatUserSerializer,
        },
    },
    set_active_model_fields, set_fields,
    state::AppState,
    utils::{
        extractors::auth::AuthAdmin,
        password::create_password,
        response::{CustomResponse, to_error_response, to_error_response_with_message},
    },
};

async fn create_retreat(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
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

async fn list_retreats(
    State(state): State<AppState>,
    Query(filter): Query<RetreatFilter>,
) -> Result<Response<Body>, Response<Body>> {
    let mut query = RetreatEntity::find();
    if filter.is_published == Some(true) {
        query = query.filter(RetreatColumn::IsPublished.eq(true));
    }

    let instances: Vec<RetreatModel> = query.clone()
        .limit(filter.limit())
        .offset(filter.offset())
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let retreat_ids: Vec<i64> = instances.iter().map(|m| m.retreat_id).collect();
    let mut serializers: Vec<ReadRetreatSerializer> =
        instances.into_iter().map(|model| model.into()).collect();

    let reviews = RetreatReviewEntity::find()
        .filter(RetreatReviewColumn::RetreatId.is_in(retreat_ids.clone()))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let mut sum_map: std::collections::HashMap<i64, f64> = std::collections::HashMap::new();
    let mut count_map: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
    for review in &reviews {
        *sum_map.entry(review.retreat_id).or_default() += review.rating;
        *count_map.entry(review.retreat_id).or_default() += 1;
    }
    let avg_map: std::collections::HashMap<i64, f64> = sum_map
        .into_iter()
        .filter_map(|(id, sum)| count_map.get(&id).map(|&count| (id, sum / count as f64)))
        .collect();
    for (serializer, id) in serializers.iter_mut().zip(&retreat_ids) {
        serializer.average_rating = avg_map.get(id).copied();
    }

    let total: u64 = query.count(&state.database).await.unwrap();
    let pagination_meta = filter.build_meta(total);
    Ok(CustomResponse::<Vec<ReadRetreatSerializer>, PaginationMeta>::builder(serializers).meta(pagination_meta).build())
}

async fn get_retreat(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
    Query(filter): Query<RetreatFilter>,
) -> Result<Response<Body>, Response<Body>> {
    let mut query = RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id));
    if filter.is_published == Some(true) {
        query = query.filter(RetreatColumn::IsPublished.eq(true));
    }

    let instance = query
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    let retreat_id = instance.retreat_id;
    let mut serializer: ReadRetreatSerializer = instance.into();

    let reviews = RetreatReviewEntity::find()
        .filter(RetreatReviewColumn::RetreatId.eq(retreat_id))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    if !reviews.is_empty() {
        let sum: f64 = reviews.iter().map(|r| r.rating).sum();
        serializer.average_rating = Some(sum / reviews.len() as f64);
    }

    Ok(CustomResponse::<ReadRetreatSerializer, ()>::builder(serializer).build())
}

async fn update_retreat(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
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
    AuthAdmin(_): AuthAdmin,
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
    AuthAdmin(_): AuthAdmin,
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
    AuthAdmin(_): AuthAdmin,
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
    AuthAdmin(_): AuthAdmin,
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

async fn list_retreat_users(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Path(retreat_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let retreat_users: Vec<RetreatUserModel> = RetreatUserEntity::find()
        .filter(RetreatUserColumn::RetreatId.eq(retreat_id))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let user_ids: Vec<i64> = retreat_users.iter().map(|ru| ru.user_id).collect();

    let users: Vec<UserModel> = UserEntity::find()
        .filter(UserColumn::UserId.is_in(user_ids))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let user_map: std::collections::HashMap<i64, &UserModel> =
        users.iter().map(|u| (u.user_id, u)).collect();

    let serializers: Vec<ReadRetreatUserSerializer> = retreat_users
        .into_iter()
        .map(|ru| {
            let user = user_map.get(&ru.user_id).expect("User should exist");
            ReadRetreatUserSerializer {
                retreat_user_id: ru.retreat_user_id,
                retreat_id: ru.retreat_id,
                user_id: ru.user_id,
                name: user.name.clone(),
                email: user.email.clone(),
                role: ru.role,
            }
        })
        .collect();

    Ok(CustomResponse::<Vec<ReadRetreatUserSerializer>, ()>::builder(serializers).build())
}

pub fn retreat_router() -> Router<AppState> {
    let router = Router::new()
        .route("/retreats/", post(create_retreat))
        .route("/retreats/", get(list_retreats))
        .route("/retreats/{retreat_id}/", get(get_retreat))
        .route("/retreats/{retreat_id}/", patch(update_retreat))
        .route("/retreats/{retreat_id}/", delete(delete_retreat))
        .route("/retreats/{retreat_id}/users/", get(list_retreat_users).post(create_retreat_user))
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
