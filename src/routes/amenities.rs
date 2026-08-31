use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{Response, StatusCode},
    routing::{delete, get, patch, post},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, TryIntoModel,
};
use sea_orm::sea_query::{Expr, extension::postgres::PgExpr};
use validator::Validate;

use crate::{
    entities_helper::{
        AmenityActiveModel, AmenityColumn, AmenityEntity, AmenityModel,
    },
    serializers::{
        amenities::{
            AmenityFilter, CreateAmenitySerializer, ReadAmenitySerializer,
            UpdateAmenitySerializer,
        },
        pagination::{Paginate, PaginationMeta},
    },
    set_active_model_fields, set_fields, state::AppState,
    utils::{
        extractors::auth::AuthAdmin,
        response::{to_error_response, to_error_response_with_message, CustomResponse},
    },
};

async fn create_amenity(
    State(state): State<AppState>,
    AuthAdmin(user): AuthAdmin,
    Json(payload): Json<CreateAmenitySerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let mut active_model: AmenityActiveModel =
        set_active_model_fields!(payload, AmenityActiveModel, { label });

    active_model.created_by = Set(Some(user.user_id));
    active_model.updated_by = Set(Some(user.user_id));

    let active_model = active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadAmenitySerializer = active_model
        .try_into_model()
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .into();

    Ok(CustomResponse::<ReadAmenitySerializer, ()>::builder(serializer)
        .message("Amenity created successfully.")
        .status_code(StatusCode::CREATED)
        .build())
}

async fn list_amenities(
    State(state): State<AppState>,
    Query(filter): Query<AmenityFilter>,
) -> Result<Response<Body>, Response<Body>> {
    let mut query = AmenityEntity::find();

    if let Some(ref search) = filter.search {
        query = query.filter(Expr::col(AmenityColumn::Label).ilike(format!("%{}%", search)));
    }

    let total: u64 = query.clone().count(&state.database).await.unwrap();

    let instances: Vec<AmenityModel> = query
        .clone()
        .limit(filter.limit())
        .offset(filter.offset())
        .order_by(AmenityColumn::AmenityId, sea_orm::Order::Desc)
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializers: Vec<ReadAmenitySerializer> =
        instances.into_iter().map(|model| model.into()).collect();

    let pagination_meta = filter.build_meta(total);
    Ok(CustomResponse::<Vec<ReadAmenitySerializer>, PaginationMeta>::builder(serializers)
        .meta(pagination_meta)
        .build())
}

async fn get_amenity(
    State(state): State<AppState>,
    Path(amenity_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instance = AmenityEntity::find()
        .filter(AmenityColumn::AmenityId.eq(amenity_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Amenity not found.", StatusCode::NOT_FOUND)
        })?;

    let serializer: ReadAmenitySerializer = instance.into();
    Ok(CustomResponse::<ReadAmenitySerializer, ()>::builder(serializer).build())
}

async fn update_amenity(
    State(state): State<AppState>,
    AuthAdmin(user): AuthAdmin,
    Path(amenity_id): Path<i64>,
    Json(payload): Json<UpdateAmenitySerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let instance = AmenityEntity::find()
        .filter(AmenityColumn::AmenityId.eq(amenity_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Amenity not found.", StatusCode::NOT_FOUND)
        })?;

    let mut active_model: AmenityActiveModel = instance.into_active_model();

    set_fields!(active_model, payload, label);
    active_model.updated_by = Set(Some(user.user_id));

    let instance = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadAmenitySerializer = instance.into();
    Ok(CustomResponse::<ReadAmenitySerializer, ()>::builder(serializer)
        .message("Amenity updated successfully.")
        .status_code(StatusCode::OK)
        .build())
}

async fn delete_amenity(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Path(amenity_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instance = AmenityEntity::find()
        .filter(AmenityColumn::AmenityId.eq(amenity_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Amenity not found.", StatusCode::NOT_FOUND)
        })?;

    let active_model: AmenityActiveModel = instance.into_active_model();
    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(CustomResponse::<(), ()>::builder({})
        .message("Amenity deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

pub fn amenity_router() -> Router<AppState> {
    Router::new()
        .route("/amenities/", post(create_amenity))
        .route("/amenities/", get(list_amenities))
        .route("/amenities/{amenity_id}/", get(get_amenity))
        .route("/amenities/{amenity_id}/", patch(update_amenity))
        .route("/amenities/{amenity_id}/", delete(delete_amenity))
}
