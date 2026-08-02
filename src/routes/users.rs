use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    routing::{delete, get, patch, post},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, ExprTrait, IntoActiveModel,
    Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TryIntoModel,
};
use sea_orm::sea_query::{Expr, extension::postgres::PgExpr};
use validator::Validate;

use crate::{
    entities_helper::{
        RetreatColumn, RetreatEntity, RetreatModel, RetreatUserColumn, RetreatUserEntity,
        RetreatUserModel, UserActiveModel, UserColumn, UserEntity, UserModel,
    },
    serializers::{
        pagination::{Paginate, PaginationMeta},
        users::{CreateUserSerializer, ReadUserSerializer, UpdateUserSerializer, UserFilter},
    },
    set_fields,
    state::AppState,
    utils::{
        extractors::auth::{AuthAdmin, AuthUserOrAdmin},
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
    Query(filter): Query<UserFilter>,
) -> Result<Response<Body>, Response<Body>> {
    let mut query = UserEntity::find();

    if let Some(ref search) = filter.search {
        query = query.filter(
            Expr::col(UserColumn::Name)
                .ilike(format!("%{}%", search))
                .or(Expr::col(UserColumn::Email)
                    .ilike(format!("%{}%", search))),
        );
    }

    if let Some(ref retreat_name) = filter.retreat_name {
        let retreat_ids: Vec<i64> = RetreatEntity::find()
            .filter(RetreatColumn::Name.ilike(format!("%{}%", retreat_name)))
            .all(&state.database)
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
            .into_iter()
            .map(|model| model.retreat_id)
            .collect();

        let user_ids: Vec<i64> = if retreat_ids.is_empty() {
            Vec::new()
        } else {
            RetreatUserEntity::find()
                .filter(RetreatUserColumn::RetreatId.is_in(retreat_ids))
                .all(&state.database)
                .await
                .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
                .into_iter()
                .map(|model| model.user_id)
                .collect()
        };

        if user_ids.is_empty() {
            query = query.filter(UserColumn::UserId.eq(-1));
        } else {
            query = query.filter(UserColumn::UserId.is_in(user_ids));
        }
    }

    match filter.sort_by.as_deref() {
        Some("name") => {
            let order = match filter.sort_order.as_deref() {
                Some("desc") => Order::Desc,
                _ => Order::Asc,
            };
            query = query.order_by(UserColumn::Name, order);
        }
        _ => {
            query = query.order_by(UserColumn::UserId, Order::Desc);
        }
    }

    let total: u64 = query.clone().count(&state.database).await.unwrap();
    let instances: Vec<UserModel> = query
        .limit(filter.limit())
        .offset(filter.offset())
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let user_ids: Vec<i64> = instances.iter().map(|model| model.user_id).collect();

    let retreat_user_rows: Vec<RetreatUserModel> = if user_ids.is_empty() {
        Vec::new()
    } else {
        RetreatUserEntity::find()
            .filter(RetreatUserColumn::UserId.is_in(user_ids.clone()))
            .all(&state.database)
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
    };

    let retreat_ids: Vec<i64> = retreat_user_rows
        .iter()
        .map(|model| model.retreat_id)
        .collect();

    let retreat_rows: Vec<RetreatModel> = if retreat_ids.is_empty() {
        Vec::new()
    } else {
        RetreatEntity::find()
            .filter(RetreatColumn::RetreatId.is_in(retreat_ids))
            .all(&state.database)
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
    };

    let retreat_name_map: std::collections::HashMap<i64, String> = retreat_rows
        .into_iter()
        .map(|model| (model.retreat_id, model.name))
        .collect();

    let mut user_retreat_names: std::collections::HashMap<i64, Vec<String>> =
        std::collections::HashMap::new();
    for row in retreat_user_rows {
        if let Some(name) = retreat_name_map.get(&row.retreat_id) {
            user_retreat_names
                .entry(row.user_id)
                .or_default()
                .push(name.clone());
        }
    }

    let serializers: Vec<ReadUserSerializer> = instances
        .into_iter()
        .map(|model| {
            let mut serializer: ReadUserSerializer = model.into();
            serializer.retreats = user_retreat_names
                .get(&serializer.user_id)
                .cloned()
                .unwrap_or_default();
            serializer
        })
        .collect();

    let pagination_meta = filter.build_meta(total);
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
    AuthUserOrAdmin(auth): AuthUserOrAdmin,
    Path(user_id): Path<i64>,
    Json(payload): Json<UpdateUserSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    if !auth.is_admin() && auth.user_id() != user_id {
        return Err(to_error_response_with_message(
            "You can only update your own profile.",
            StatusCode::FORBIDDEN,
        ));
    }
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
    AuthAdmin(_): AuthAdmin,
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
