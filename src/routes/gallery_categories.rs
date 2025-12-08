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
        GalleryCategoriesActiveModel, GalleryCategoriesColumn, GalleryCategoriesEntity,
        GalleryCategoriesModel,
    },
    serializers::gallery_categories::{
        CreateGalleryCategorySerializer, ReadGalleryCategorySerializer,
        UpdateGalleryCategorySerializer,
    },
    set_active_model_fields, set_fields,
    state::AppState,
    utils::{
        extractors::auth::AuthAdmin,
        response::{CustomResponse, to_error_response, to_error_response_with_message},
    },
};

async fn create_gallery_category(
    State(state): State<AppState>,
    AuthAdmin(user): AuthAdmin,
    Json(payload): Json<CreateGalleryCategorySerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let mut active_model: GalleryCategoriesActiveModel = set_active_model_fields!(payload, GalleryCategoriesActiveModel, {
        name,
    });
    active_model.created_by = Set(Some(user.user_id));
    active_model.updated_by = Set(Some(user.user_id));

    // save Retreat
    let active_model: GalleryCategoriesActiveModel = active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    // convert to ReadRetreatSerializer serializer
    let serializer: ReadGalleryCategorySerializer = active_model
        .try_into_model()
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .into();
    Ok(CustomResponse::<ReadGalleryCategorySerializer, ()>::builder(serializer)
        .message("Gallery category created successfully.")
        .status_code(StatusCode::CREATED)
        .build())
}

async fn list_gallery_category(
    State(state): State<AppState>,
) -> Result<Response<Body>, Response<Body>> {
    let instances: Vec<GalleryCategoriesModel> = GalleryCategoriesEntity::find()
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    // Convert model to serializer
    let serializers: Vec<ReadGalleryCategorySerializer> =
        instances.into_iter().map(|model| model.into()).collect();

    Ok(CustomResponse::<Vec<ReadGalleryCategorySerializer>, ()>::builder(serializers).build())
}

async fn update_gallery_category(
    State(state): State<AppState>,
    AuthAdmin(user): AuthAdmin,
    Path(gallery_category_id): Path<i64>,
    Json(payload): Json<UpdateGalleryCategorySerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let instance: GalleryCategoriesModel = GalleryCategoriesEntity::find()
        .filter(GalleryCategoriesColumn::GalleryCategoryId.eq(gallery_category_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Gallery Category not found.", StatusCode::NOT_FOUND)
        })?;

    let mut active_model: GalleryCategoriesActiveModel = instance.into_active_model();

    set_fields!(active_model, payload, name);

    active_model.updated_by = Set(Some(user.user_id));

    // Save the updated Retreat
    let instance: GalleryCategoriesModel = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert to serializer
    let serializer: ReadGalleryCategorySerializer = instance.into();

    Ok(CustomResponse::<ReadGalleryCategorySerializer, ()>::builder(serializer).build())
}

async fn delete_gallery_category(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Path(gallery_category_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instance: GalleryCategoriesModel = GalleryCategoriesEntity::find()
        .filter(GalleryCategoriesColumn::GalleryCategoryId.eq(gallery_category_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Gallery Category not found.", StatusCode::NOT_FOUND)
        })?;
    // Convert to ActiveModel for editing
    let active_model: GalleryCategoriesActiveModel = instance.into_active_model();

    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert model to serializer
    Ok(CustomResponse::<(), ()>::builder({})
        .message("Gallery category deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

pub fn gallery_category_router() -> Router<AppState> {
    let router = Router::new()
        .route("/gallery-categories/", post(create_gallery_category))
        .route("/gallery-categories/", get(list_gallery_category))
        .route(
            "/gallery-categories/{gallery_category_id}/",
            patch(update_gallery_category),
        )
        .route(
            "/gallery-categories/{gallery_category_id}/",
            delete(delete_gallery_category),
        );
    return router;
}
