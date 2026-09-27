use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
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
        RetreatPackagesActiveModel, RetreatPackagesColumn, RetreatPackagesEntity,
        RetreatPackagesModel, RetreatRoomTypesColumn, RetreatRoomTypesEntity,
        RetreatRoomTypesModel,
    },
    serializers::retreat_packages::{
        CreatePackageSerializer, ReadPackageSerializer, UpdatePackageSerializer,
    },
    set_active_model_fields, set_fields,
    state::AppState,
    utils::{
        extractors::auth::{AuthAdminOrRetreatUser, ensure_retreat_membership},
        response::{CustomResponse, to_error_response, to_error_response_with_message},
    },
};

fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .map(|c| if c == ' ' { '-' } else { c })
        .collect::<String>()
}

/// Slugs only collide within a retreat, so a second "Wellness Weekend" at
/// another property is fine.
async fn ensure_unique_slug(
    db: &sea_orm::DatabaseConnection,
    retreat_id: i64,
    base_slug: &str,
) -> Result<String, Response<Body>> {
    let mut slug = base_slug.to_string();
    let mut counter = 0;
    loop {
        let exists: u64 = RetreatPackagesEntity::find()
            .filter(RetreatPackagesColumn::RetreatId.eq(retreat_id))
            .filter(RetreatPackagesColumn::Slug.eq(&slug))
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

/// A package may only point at a room type belonging to the same retreat, so a
/// package can never advertise another property's room.
async fn validate_room_type_belongs_to_retreat(
    db: &sea_orm::DatabaseConnection,
    retreat_id: i64,
    room_type_id: Option<i64>,
) -> Result<(), Response<Body>> {
    let Some(room_type_id) = room_type_id else {
        return Ok(());
    };
    let exists: u64 = RetreatRoomTypesEntity::find()
        .filter(RetreatRoomTypesColumn::RetreatRoomTypeId.eq(room_type_id))
        .filter(RetreatRoomTypesColumn::RetreatId.eq(retreat_id))
        .count(db)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    if exists == 0 {
        return Err(to_error_response_with_message(
            "Selected room type does not belong to this retreat.",
            StatusCode::BAD_REQUEST,
        ));
    }
    Ok(())
}

/// Loads `room_type_id -> name` for the given packages in one query and attaches
/// the names to the serializers, avoiding an N+1 lookup per package.
async fn attach_room_type_names(
    state: &AppState,
    instances: Vec<RetreatPackagesModel>,
) -> Vec<ReadPackageSerializer> {
    let mut names: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
    let ids: Vec<i64> = instances
        .iter()
        .filter_map(|model| model.room_type_id)
        .collect();

    if !ids.is_empty() {
        // A failed lookup only costs us the denormalized name, so it degrades
        // quietly instead of failing the whole list.
        let room_types: Vec<RetreatRoomTypesModel> =
            match RetreatRoomTypesEntity::find()
                .filter(RetreatRoomTypesColumn::RetreatRoomTypeId.is_in(ids))
                .all(&state.database)
                .await
            {
                Ok(room_types) => room_types,
                Err(error) => {
                    eprintln!("Warning: failed to resolve package room type names: {error}");
                    Vec::new()
                }
            };
        for RetreatRoomTypesModel {
            retreat_room_type_id,
            name,
            ..
        } in room_types
        {
            names.insert(retreat_room_type_id, name);
        }
    }

    instances
        .into_iter()
        .map(|model| {
            let room_type_name: Option<String> =
                model.room_type_id.and_then(|id| names.get(&id).cloned());
            let serializer: ReadPackageSerializer = model.into();
            serializer.with_room_type_name(room_type_name)
        })
        .collect()
}

async fn create_package(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path(retreat_id): Path<i64>,
    Json(payload): Json<CreatePackageSerializer>,
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
    validate_room_type_belongs_to_retreat(&state.database, retreat_id, payload.room_type_id).await?;

    let base_slug = slugify(&name);
    if base_slug.is_empty() {
        return Err(to_error_response_with_message(
            "Could not generate a slug from the name. Please use letters or numbers.",
            StatusCode::BAD_REQUEST,
        ));
    }
    let slug = ensure_unique_slug(&state.database, retreat_id, &base_slug).await?;

    let mut active_model: RetreatPackagesActiveModel =
        set_active_model_fields!(payload, RetreatPackagesActiveModel, {
            description,
            duration_nights,
            includes,
            price,
            is_featured,
            display_order,
        });
    active_model.retreat_id = Set(retreat_id);
    active_model.room_type_id = Set(payload.room_type_id);
    active_model.name = Set(name);
    active_model.slug = Set(slug);
    active_model.created_by = Set(Some(principal.user_id()));
    active_model.updated_by = Set(Some(principal.user_id()));

    let instance: RetreatPackagesModel = active_model
        .insert(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let room_type_name: Option<String> = match instance.room_type_id {
        Some(room_type_id) => RetreatRoomTypesEntity::find_by_id(room_type_id)
            .one(&state.database)
            .await
            .ok()
            .flatten()
            .map(|room_type| room_type.name),
        None => None,
    };

    let serializer: ReadPackageSerializer = ReadPackageSerializer::from(instance).with_room_type_name(room_type_name);
    Ok(CustomResponse::<ReadPackageSerializer, ()>::builder(serializer)
        .message("Package created successfully.")
        .status_code(StatusCode::CREATED)
        .build())
}

async fn list_packages(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instances: Vec<RetreatPackagesModel> = RetreatPackagesEntity::find()
        .filter(RetreatPackagesColumn::RetreatId.eq(retreat_id))
        .order_by_asc(RetreatPackagesColumn::DisplayOrder)
        .order_by_asc(RetreatPackagesColumn::RetreatPackageId)
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializers: Vec<ReadPackageSerializer> = attach_room_type_names(&state, instances).await;

    Ok(CustomResponse::<Vec<ReadPackageSerializer>, ()>::builder(serializers).build())
}

async fn get_package(
    State(state): State<AppState>,
    Path((retreat_id, retreat_package_id)): Path<(i64, i64)>,
) -> Result<Response<Body>, Response<Body>> {
    let instance: RetreatPackagesModel = RetreatPackagesEntity::find()
        .filter(RetreatPackagesColumn::RetreatPackageId.eq(retreat_package_id))
        .filter(RetreatPackagesColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("Package not found.", StatusCode::NOT_FOUND))?;

    let room_type_name: Option<String> = match instance.room_type_id {
        Some(room_type_id) => RetreatRoomTypesEntity::find_by_id(room_type_id)
            .one(&state.database)
            .await
            .ok()
            .flatten()
            .map(|room_type| room_type.name),
        None => None,
    };

    let serializer: ReadPackageSerializer = ReadPackageSerializer::from(instance).with_room_type_name(room_type_name);
    Ok(CustomResponse::<ReadPackageSerializer, ()>::builder(serializer).build())
}

async fn update_package(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path((retreat_id, retreat_package_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdatePackageSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    ensure_retreat_membership(&state.database, &principal, retreat_id).await?;
    // Re-check on update so a package cannot be re-pointed at a foreign room.
    if let Some(room_type_id) = payload.room_type_id {
        validate_room_type_belongs_to_retreat(&state.database, retreat_id, room_type_id).await?;
    }

    let instance: RetreatPackagesModel = RetreatPackagesEntity::find()
        .filter(RetreatPackagesColumn::RetreatPackageId.eq(retreat_package_id))
        .filter(RetreatPackagesColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("Package not found.", StatusCode::NOT_FOUND))?;

    // Slugs are derived once at creation and left stable afterwards.
    if payload.name.as_ref().is_some_and(|name| name.trim().is_empty()) {
        return Err(to_error_response_with_message(
            "Name cannot be empty.",
            StatusCode::BAD_REQUEST,
        ));
    }

    let mut active_model: RetreatPackagesActiveModel = instance.into_active_model();
    set_fields!(active_model, payload, description, room_type_id, duration_nights, includes, price, is_featured, display_order);
    if let Some(name) = payload.name {
        active_model.name = Set(name.trim().to_string());
    }
    active_model.updated_by = Set(Some(principal.user_id()));

    let instance: RetreatPackagesModel = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let room_type_name: Option<String> = match instance.room_type_id {
        Some(room_type_id) => RetreatRoomTypesEntity::find_by_id(room_type_id)
            .one(&state.database)
            .await
            .ok()
            .flatten()
            .map(|room_type| room_type.name),
        None => None,
    };

    let serializer: ReadPackageSerializer = ReadPackageSerializer::from(instance).with_room_type_name(room_type_name);
    Ok(CustomResponse::<ReadPackageSerializer, ()>::builder(serializer)
        .message("Package updated successfully.")
        .build())
}

async fn delete_package(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path((retreat_id, retreat_package_id)): Path<(i64, i64)>,
) -> Result<Response<Body>, Response<Body>> {
    ensure_retreat_membership(&state.database, &principal, retreat_id).await?;

    let instance: RetreatPackagesModel = RetreatPackagesEntity::find()
        .filter(RetreatPackagesColumn::RetreatPackageId.eq(retreat_package_id))
        .filter(RetreatPackagesColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("Package not found.", StatusCode::NOT_FOUND))?;

    let active_model: RetreatPackagesActiveModel = instance.into_active_model();
    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(CustomResponse::<(), ()>::builder({})
        .message("Package deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

pub fn package_router() -> Router<AppState> {
    Router::new()
        .route(
            "/retreats/{retreat_id}/packages/",
            post(create_package).get(list_packages),
        )
        .route(
            "/retreats/{retreat_id}/packages/{retreat_package_id}/",
            get(get_package)
                .patch(update_package)
                .delete(delete_package),
        )
}
