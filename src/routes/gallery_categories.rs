use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::{patch, post},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter, TryIntoModel,
};
use validator::Validate;

use crate::{
    entities_helper::{
        GalleryCategoriesActiveModel, GalleryCategoriesColumn, GalleryCategoriesEntity,
        GalleryCategoriesModel, RetreatColumn, RetreatEntity, RetreatGalleriesColumn,
        RetreatGalleriesEntity,
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
    Path(retreat_id): Path<i64>,
    Json(payload): Json<CreateGalleryCategorySerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND))?;

    let mut active_model: GalleryCategoriesActiveModel = set_active_model_fields!(payload, GalleryCategoriesActiveModel, {
        name,
    });
    active_model.retreat_id = Set(retreat_id);
    active_model.created_by = Set(Some(user.user_id));
    active_model.updated_by = Set(Some(user.user_id));

    let active_model: GalleryCategoriesActiveModel = active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
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
    Path(retreat_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instances: Vec<GalleryCategoriesModel> = GalleryCategoriesEntity::find()
        .filter(GalleryCategoriesColumn::RetreatId.eq(retreat_id))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    let serializers: Vec<ReadGalleryCategorySerializer> =
        instances.into_iter().map(|model| model.into()).collect();

    Ok(CustomResponse::<Vec<ReadGalleryCategorySerializer>, ()>::builder(serializers).build())
}

async fn update_gallery_category(
    State(state): State<AppState>,
    AuthAdmin(user): AuthAdmin,
    Path((retreat_id, gallery_category_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateGalleryCategorySerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let instance: GalleryCategoriesModel = GalleryCategoriesEntity::find()
        .filter(GalleryCategoriesColumn::GalleryCategoryId.eq(gallery_category_id))
        .filter(GalleryCategoriesColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Gallery Category not found.", StatusCode::NOT_FOUND)
        })?;

    let mut active_model: GalleryCategoriesActiveModel = instance.into_active_model();

    set_fields!(active_model, payload, name);

    active_model.updated_by = Set(Some(user.user_id));

    let instance: GalleryCategoriesModel = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadGalleryCategorySerializer = instance.into();

    Ok(CustomResponse::<ReadGalleryCategorySerializer, ()>::builder(serializer).build())
}

async fn delete_gallery_category(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Path((retreat_id, gallery_category_id)): Path<(i64, i64)>,
) -> Result<Response<Body>, Response<Body>> {
    let instance: GalleryCategoriesModel = GalleryCategoriesEntity::find()
        .filter(GalleryCategoriesColumn::GalleryCategoryId.eq(gallery_category_id))
        .filter(GalleryCategoriesColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Gallery Category not found.", StatusCode::NOT_FOUND)
        })?;

    let image_count: u64 = RetreatGalleriesEntity::find()
        .filter(RetreatGalleriesColumn::GalleryCategoryId.eq(gallery_category_id))
        .count(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    if image_count > 0 {
        return Err(to_error_response_with_message(
            &format!("Cannot delete category with {} associated image(s). Reassign them first.", image_count),
            StatusCode::BAD_REQUEST,
        ));
    }

    let active_model: GalleryCategoriesActiveModel = instance.into_active_model();

    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(CustomResponse::<(), ()>::builder({})
        .message("Gallery category deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

pub fn gallery_category_router() -> Router<AppState> {
    Router::new()
        .route(
            "/retreats/{retreat_id}/gallery-categories/",
            post(create_gallery_category).get(list_gallery_category),
        )
        .route(
            "/retreats/{retreat_id}/gallery-categories/{gallery_category_id}/",
            patch(update_gallery_category).delete(delete_gallery_category),
        )
}
