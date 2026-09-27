use axum::{
    Json, Router,
    body::Body,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::Response,
    routing::{get, post},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter, QueryOrder,
};
use validator::Validate;

use crate::{
    entities_helper::{
        RetreatRoomTypesActiveModel, RetreatRoomTypesColumn, RetreatRoomTypesEntity,
        RetreatRoomTypesModel,
    },
    serializers::retreat_room_types::{
        CreateRoomTypeSerializer, ReadRoomTypeSerializer, UpdateRoomTypeSerializer,
    },
    set_active_model_fields, set_fields,
    state::AppState,
    utils::{
        extractors::auth::{AuthAdminOrRetreatUser, ensure_retreat_membership},
        response::{CustomResponse, to_error_response, to_error_response_with_message},
        storage::{self, read_image_with_headers},
    },
};

/// Sub-directory room type photos are written to. Kept separate from the
/// gallery so deleting a gallery image can never remove a room photo.
const ROOM_TYPE_IMAGE_DIR: &str = "retreat/room_types";

fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .map(|c| if c == ' ' { '-' } else { c })
        .collect::<String>()
}

/// Slugs only collide within a retreat, so a second "Garden Suite" at another
/// property is fine.
async fn ensure_unique_slug(
    db: &sea_orm::DatabaseConnection,
    retreat_id: i64,
    base_slug: &str,
) -> Result<String, Response<Body>> {
    let mut slug = base_slug.to_string();
    let mut counter = 0;
    loop {
        let exists: u64 = RetreatRoomTypesEntity::find()
            .filter(RetreatRoomTypesColumn::RetreatId.eq(retreat_id))
            .filter(RetreatRoomTypesColumn::Slug.eq(&slug))
            .count(db)
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
        if exists == 0 {
            return Ok(slug);
        }
        counter += 1;
        slug = format!("{}-{}", base_slug, counter);
    }
}

async fn create_room_type(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path(retreat_id): Path<i64>,
    Json(payload): Json<CreateRoomTypeSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let name: String = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(to_error_response_with_message(
            "Name is required.",
            StatusCode::BAD_REQUEST,
        ));
    }

    // Tenant scope: retreat staff may only manage their own retreat's offerings.
    // Global admins bypass.
    ensure_retreat_membership(&state.database, &principal, retreat_id).await?;

    let base_slug = slugify(&name);
    if base_slug.is_empty() {
        return Err(to_error_response_with_message(
            "Could not generate a slug from the name. Please use letters or numbers.",
            StatusCode::BAD_REQUEST,
        ));
    }
    let slug = ensure_unique_slug(&state.database, retreat_id, &base_slug).await?;

    let mut active_model: RetreatRoomTypesActiveModel =
        set_active_model_fields!(payload, RetreatRoomTypesActiveModel, {
            description,
            size_sqm,
            max_guests,
            bed_configuration,
            amenities,
            price_per_night,
            is_featured,
            display_order,
        });
    active_model.retreat_id = Set(retreat_id);
    active_model.name = Set(name);
    active_model.slug = Set(slug);
    active_model.created_by = Set(Some(principal.user_id()));
    active_model.updated_by = Set(Some(principal.user_id()));

    let instance: RetreatRoomTypesModel = active_model
        .insert(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadRoomTypeSerializer = instance.into();
    Ok(CustomResponse::<ReadRoomTypeSerializer, ()>::builder(serializer)
        .message("Room type created successfully.")
        .status_code(StatusCode::CREATED)
        .build())
}

async fn list_room_types(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instances: Vec<RetreatRoomTypesModel> = RetreatRoomTypesEntity::find()
        .filter(RetreatRoomTypesColumn::RetreatId.eq(retreat_id))
        .order_by_asc(RetreatRoomTypesColumn::DisplayOrder)
        .order_by_asc(RetreatRoomTypesColumn::RetreatRoomTypeId)
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    let serializers: Vec<ReadRoomTypeSerializer> =
        instances.into_iter().map(|model| model.into()).collect();

    Ok(CustomResponse::<Vec<ReadRoomTypeSerializer>, ()>::builder(serializers).build())
}

async fn get_room_type(
    State(state): State<AppState>,
    Path((retreat_id, retreat_room_type_id)): Path<(i64, i64)>,
) -> Result<Response<Body>, Response<Body>> {
    let instance: RetreatRoomTypesModel = RetreatRoomTypesEntity::find()
        .filter(RetreatRoomTypesColumn::RetreatRoomTypeId.eq(retreat_room_type_id))
        .filter(RetreatRoomTypesColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("Room type not found.", StatusCode::NOT_FOUND))?;

    let serializer: ReadRoomTypeSerializer = instance.into();
    Ok(CustomResponse::<ReadRoomTypeSerializer, ()>::builder(serializer).build())
}

async fn update_room_type(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path((retreat_id, retreat_room_type_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateRoomTypeSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    ensure_retreat_membership(&state.database, &principal, retreat_id).await?;

    let instance: RetreatRoomTypesModel = RetreatRoomTypesEntity::find()
        .filter(RetreatRoomTypesColumn::RetreatRoomTypeId.eq(retreat_room_type_id))
        .filter(RetreatRoomTypesColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("Room type not found.", StatusCode::NOT_FOUND))?;

    // Slugs are derived once at creation and left stable afterwards so existing
    // links and package references keep resolving.
    if payload.name.as_ref().is_some_and(|name| name.trim().is_empty()) {
        return Err(to_error_response_with_message(
            "Name cannot be empty.",
            StatusCode::BAD_REQUEST,
        ));
    }

    let mut active_model: RetreatRoomTypesActiveModel = instance.into_active_model();
    set_fields!(active_model, payload, description, size_sqm, max_guests, bed_configuration, amenities, price_per_night, is_featured, display_order);
    if let Some(name) = payload.name {
        active_model.name = Set(name.trim().to_string());
    }
    active_model.updated_by = Set(Some(principal.user_id()));

    let instance: RetreatRoomTypesModel = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadRoomTypeSerializer = instance.into();
    Ok(CustomResponse::<ReadRoomTypeSerializer, ()>::builder(serializer)
        .message("Room type updated successfully.")
        .build())
}

async fn delete_room_type(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path((retreat_id, retreat_room_type_id)): Path<(i64, i64)>,
) -> Result<Response<Body>, Response<Body>> {
    ensure_retreat_membership(&state.database, &principal, retreat_id).await?;

    let instance: RetreatRoomTypesModel = RetreatRoomTypesEntity::find()
        .filter(RetreatRoomTypesColumn::RetreatRoomTypeId.eq(retreat_room_type_id))
        .filter(RetreatRoomTypesColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("Room type not found.", StatusCode::NOT_FOUND))?;

    // Packages keep existing and fall back to no linked room (the FK is
    // ON DELETE SET NULL), so no orphan guard is needed here.
    let active_model: RetreatRoomTypesActiveModel = instance.clone().into_active_model();
    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Best effort cleanup; a missing file must not fail the delete.
    if let Some(image_path) = instance.image_path {
        storage::remove_image(image_path).await;
    }

    Ok(CustomResponse::<(), ()>::builder({})
        .message("Room type deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

async fn upload_room_type_image(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path((retreat_id, retreat_room_type_id)): Path<(i64, i64)>,
    mut multipart: Multipart,
) -> Result<Response<Body>, Response<Body>> {
    ensure_retreat_membership(&state.database, &principal, retreat_id).await?;

    let instance: RetreatRoomTypesModel = RetreatRoomTypesEntity::find()
        .filter(RetreatRoomTypesColumn::RetreatRoomTypeId.eq(retreat_room_type_id))
        .filter(RetreatRoomTypesColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("Room type not found.", StatusCode::NOT_FOUND))?;

    let mut image_path: Option<String> = None;
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if field.name().unwrap_or("") == "image" {
            let file_name: String = field.file_name().unwrap_or("room.jpg").to_string();
            let Ok(file_content) = field.bytes().await else {
                return Err(to_error_response_with_message(
                    "Failed to read the uploaded image.",
                    StatusCode::BAD_REQUEST,
                ));
            };
            image_path = Some(
                storage::store_image(
                    file_content,
                    file_name,
                    ROOM_TYPE_IMAGE_DIR,
                    instance.image_path.clone(),
                )
                .await,
            );
        }
    }

    let image_path = image_path.ok_or_else(|| {
        to_error_response_with_message("Image file is required.", StatusCode::BAD_REQUEST)
    })?;

    let mut active_model: RetreatRoomTypesActiveModel = instance.into_active_model();
    active_model.image_path = Set(Some(image_path));
    active_model.updated_by = Set(Some(principal.user_id()));

    let instance: RetreatRoomTypesModel = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadRoomTypeSerializer = instance.into();
    Ok(CustomResponse::<ReadRoomTypeSerializer, ()>::builder(serializer)
        .message("Room type image uploaded successfully.")
        .build())
}

async fn get_room_type_image(
    State(state): State<AppState>,
    Path((retreat_id, retreat_room_type_id)): Path<(i64, i64)>,
) -> Result<Response<Body>, Response<Body>> {
    let instance: RetreatRoomTypesModel = RetreatRoomTypesEntity::find()
        .filter(RetreatRoomTypesColumn::RetreatRoomTypeId.eq(retreat_room_type_id))
        .filter(RetreatRoomTypesColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("Room type not found.", StatusCode::NOT_FOUND))?;

    let image_path = instance.image_path.ok_or_else(|| {
        to_error_response_with_message("Image not found.", StatusCode::NOT_FOUND)
    })?;

    let (bytes, headers) = read_image_with_headers(image_path)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let mut builder = Response::builder().status(StatusCode::OK);
    for (key, value) in headers.iter() {
        builder = builder.header(key, value);
    }
    Ok(builder.body(Body::from(bytes)).unwrap())
}

pub fn room_type_router() -> Router<AppState> {
    Router::new()
        .route(
            "/retreats/{retreat_id}/room-types/",
            post(create_room_type).get(list_room_types),
        )
        .route(
            "/retreats/{retreat_id}/room-types/{retreat_room_type_id}/",
            get(get_room_type)
                .patch(update_room_type)
                .delete(delete_room_type),
        )
        .route(
            "/retreats/{retreat_id}/room-types/{retreat_room_type_id}/image/",
            post(upload_room_type_image).get(get_room_type_image),
        )
}
