use axum::{
    Json, Router,
    body::Body,
    extract::{Multipart, Path, State},
    http::{Response, StatusCode},
    routing::{delete, get, patch, post},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    TryIntoModel,
};
use validator::Validate;

use crate::{
    entities_helper::{CategoryActiveModel, CategoryColumn, CategoryEntity, CategoryModel}, serializers::categories::{
        CreateCategorySerializer, ReadCategorySerializer, UpdateCategorySerializer,
    }, set_active_model_fields, set_fields, state::AppState, utils::{
        extractors::auth::AuthAdmin,
        response::{to_error_response, to_error_response_with_message, CustomResponse},
        storage::{self, read_image_with_headers},
    },
};

async fn create_category(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Json(payload): Json<CreateCategorySerializer>,
) -> Result<Response<Body>, Response<Body>> {
    println!("{:?}", payload);
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;
    let active_model: CategoryActiveModel = set_active_model_fields!(payload, CategoryActiveModel, {
        name,
        description
    });
    // save category
    let active_model: CategoryActiveModel = active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    // convert to ReadCategorySerializer serializer
    let serializer: ReadCategorySerializer = active_model
        .try_into_model()
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .into();
    Ok(CustomResponse::<ReadCategorySerializer, ()>::builder(serializer)
        .message("Category created successfully.")
        .status_code(StatusCode::CREATED)
        .build())
}

async fn list_categories(State(state): State<AppState>) -> Result<Response<Body>, Response<Body>> {
    // Query a single record
    let instances: Vec<CategoryModel> = CategoryEntity::find()
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    // Convert model to serializer
    let serializers: Vec<ReadCategorySerializer> =
        instances.into_iter().map(|model| model.into()).collect();
    Ok(CustomResponse::<Vec<ReadCategorySerializer>, ()>::builder(serializers).build())
}

async fn get_category(
    State(state): State<AppState>,
    Path(category_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    // Query a single record
    let instance = CategoryEntity::find()
        .filter(CategoryColumn::CategoryId.eq(category_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Category not found.", StatusCode::NOT_FOUND)
        })?;

    // Convert model to serializer
    let serializer: ReadCategorySerializer = instance.into();
    Ok(CustomResponse::<ReadCategorySerializer, ()>::builder(serializer).build())
}

async fn update_category(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Path(category_id): Path<i64>,
    Json(payload): Json<UpdateCategorySerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload.validate().map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;
    // Find existing category
    let instance = CategoryEntity::find()
        .filter(CategoryColumn::CategoryId.eq(category_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Category not found.", StatusCode::NOT_FOUND)
        })?;

    // Convert to ActiveModel for editing
    let mut active_model: CategoryActiveModel = instance.into_active_model();

    set_fields!(
        active_model,
        payload,
        name,
        description
    );

    // Save the updated category
    let instance = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert to serializer
    let serializer: ReadCategorySerializer = instance.into();

    // Return success
    Ok(CustomResponse::<ReadCategorySerializer, ()>::builder(serializer)
        .message("Category updated successfully.")
        .status_code(StatusCode::OK)
        .build())
}

async fn delete_category(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Path(category_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    // Query a single record
    let instance = CategoryEntity::find()
        .filter(CategoryColumn::CategoryId.eq(category_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Category not found.", StatusCode::NOT_FOUND)
        })?;

    // Convert to ActiveModel for editing
    let active_model: CategoryActiveModel = instance.into_active_model();

    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert model to serializer
    Ok(CustomResponse::<(), ()>::builder({})
        .message("Category deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

async fn upload_category_thumbnail(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Path(category_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Response<Body>, Response<Body>> {
    let instance = CategoryEntity::find()
        .filter(CategoryColumn::CategoryId.eq(category_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Category not found.", StatusCode::NOT_FOUND)
        })?;

    let mut image_path: Option<String> = None;
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if field.name().unwrap_or("") == "image" {
            let file_name = field.file_name().unwrap().to_string();
            let file_content = field.bytes().await.unwrap();
            image_path = Some(
                storage::store_image(
                    file_content,
                    file_name,
                    "category/thumbnail",
                    instance.thumbnail_image.clone(),
                )
                .await,
            );
        }
    }

    let image_path = image_path.ok_or_else(|| {
        to_error_response_with_message("Image file is required.", StatusCode::BAD_REQUEST)
    })?;

    let mut active_model: CategoryActiveModel = instance.into_active_model();
    active_model.thumbnail_image = Set(Some(image_path));

    let instance = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadCategorySerializer = instance.into();
    Ok(CustomResponse::<ReadCategorySerializer, ()>::builder(serializer)
        .message("Thumbnail uploaded successfully.")
        .build())
}

async fn get_category_thumbnail_image(
    State(state): State<AppState>,
    Path(category_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instance = CategoryEntity::find()
        .filter(CategoryColumn::CategoryId.eq(category_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Category not found.", StatusCode::NOT_FOUND)
        })?;

    let image_path = instance.thumbnail_image.ok_or_else(|| {
        to_error_response_with_message("Thumbnail not found.", StatusCode::NOT_FOUND)
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

pub fn category_router() -> Router<AppState> {
    let router = Router::new()
        .route("/categories/", post(create_category))
        .route("/categories/", get(list_categories))
        .route("/categories/{category_id}/", get(get_category))
        .route("/categories/{category_id}/", patch(update_category))
        .route("/categories/{category_id}/", delete(delete_category))
        .route("/categories/{category_id}/thumbnail/", post(upload_category_thumbnail))
        .route("/categories/{category_id}/thumbnail/image/", get(get_category_thumbnail_image));
    return router;
}
