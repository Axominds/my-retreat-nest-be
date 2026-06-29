use std::error::Error;

use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::{delete, get, patch, post},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter, QuerySelect, TryIntoModel,
};

use crate::{
    entities_helper::{
        GalleryCategoriesColumn, GalleryCategoriesEntity, RetreatColumn, RetreatEntity,
        RetreatGalleriesActiveModel, RetreatGalleriesColumn, RetreatGalleriesEntity,
        RetreatGalleriesModel,
    },
    serializers::{
        pagination::{Pagination, PaginationMeta},
        retreat_galleries::ReadRetreatGallerySerializer,
    },
    state::AppState,
    utils::{
        extractors::auth::AuthUser,
        response::{CustomResponse, to_error_response, to_error_response_with_message},
        storage::{
            read_retreat_gallery_with_headers, remove_retreat_gallery, store_retreat_gallery,
        },
    },
};

async fn create_retreat_gallery(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(retreat_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Response<Body>, Response<Body>> {
    // Find existing Retreat
    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    let mut caption: Option<String> = None;
    let mut image_path: String = "".to_string();
    let mut gallery_category_id: Option<i64> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "caption" => {
                if let Ok(value) = field.text().await {
                    caption = Some(value);
                }
            }
            "image" => {
                let file_name: String = field.file_name().unwrap().to_string();
                let file_content: Bytes = field.bytes().await.unwrap();
                image_path = store_retreat_gallery(file_content, file_name, None).await;
            }
            "gallery_category_id" => {
                if let Ok(value) = field.text().await {
                    let gallery_category_id_i64 = value.parse::<i64>().unwrap();
                    GalleryCategoriesEntity::find()
                        .filter(
                            GalleryCategoriesColumn::GalleryCategoryId.eq(gallery_category_id_i64),
                        )
                        .one(&state.database)
                        .await
                        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
                        .ok_or_else(|| {
                            to_error_response_with_message(
                                "Gallery Category not found.",
                                StatusCode::NOT_FOUND,
                            )
                        })?;
                    gallery_category_id = Some(gallery_category_id_i64)
                }
            }
            _ => {}
        }
    }
    let active_model: RetreatGalleriesActiveModel = RetreatGalleriesActiveModel {
        caption: Set(caption),
        image_path: Set(image_path),
        retreat_id: Set(retreat_id),
        gallery_category_id: Set(gallery_category_id),
        created_by: Set(Some(user.user_id)),
        updated_by: Set(Some(user.user_id)),
        ..Default::default()
    };

    // save user
    let active_model: RetreatGalleriesActiveModel = active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // convert to ReadUserSerializer serializer
    let serializer: ReadRetreatGallerySerializer = active_model
        .try_into_model()
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .into();

    Ok(
        CustomResponse::<ReadRetreatGallerySerializer, ()>::builder(serializer)
            .status_code(StatusCode::CREATED)
            .build(),
    )
}

async fn list_retreat_gallery(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
    Query(pagination): Query<Pagination>,
) -> Result<Response<Body>, Response<Body>> {
    // Find existing Retreat
    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    let page_size: u64 = pagination.limit();
    let instances: Vec<RetreatGalleriesModel> = RetreatGalleriesEntity::find()
        .filter(RetreatGalleriesColumn::RetreatId.eq(retreat_id))
        .limit(page_size)
        .offset(pagination.offset())
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert model to serializer
    let serializers: Vec<ReadRetreatGallerySerializer> =
        instances.into_iter().map(|model| model.into()).collect();

    let total: u64 = RetreatGalleriesEntity::find()
        .filter(RetreatGalleriesColumn::RetreatId.eq(retreat_id))
        .count(&state.database)
        .await
        .unwrap();
    let page: u64 = pagination.page();
    let total_pages: u64 = (total + page_size - 1) / page_size;
    let pagination_meta: PaginationMeta =
        PaginationMeta::build(total, total_pages, page_size, page);

    Ok(
        CustomResponse::<Vec<ReadRetreatGallerySerializer>, PaginationMeta>::builder(serializers)
            .meta(pagination_meta)
            .build(),
    )
}

async fn update_retreat_gallery(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
    Path((retreat_id, gallery_id)): Path<(i64, i64)>,
    mut multipart: Multipart,
) -> Result<Response<Body>, Response<Body>> {
    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    let instance: RetreatGalleriesModel = RetreatGalleriesEntity::find()
        .filter(RetreatGalleriesColumn::GalleryId.eq(gallery_id))
        .filter(RetreatGalleriesColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat gallery not found.", StatusCode::NOT_FOUND)
        })?;
    let image_path: String = instance.image_path.clone();
    // Convert to ActiveModel for editing
    let mut active_model: RetreatGalleriesActiveModel = instance.into_active_model();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "caption" => {
                if let Ok(value) = field.text().await {
                    let caption: Option<String> = Some(value);
                    active_model.caption = Set(caption);
                }
            }
            "image" => {
                let file_name: String = field.file_name().unwrap().to_string();
                let file_content: Bytes = field.bytes().await.unwrap();
                let image_path: String =
                    store_retreat_gallery(file_content, file_name, Some(image_path.clone())).await;
                active_model.image_path = Set(image_path);
            }
            "gallery_category_id" => {
                if let Ok(value) = field.text().await {
                    let gallery_category_id_i64 = value.parse::<i64>().unwrap();
                    GalleryCategoriesEntity::find()
                        .filter(
                            GalleryCategoriesColumn::GalleryCategoryId.eq(gallery_category_id_i64),
                        )
                        .one(&state.database)
                        .await
                        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
                        .ok_or_else(|| {
                            to_error_response_with_message(
                                "Gallery Category not found.",
                                StatusCode::NOT_FOUND,
                            )
                        })?;
                    let gallery_category_id: Option<i64> = Some(gallery_category_id_i64);
                    active_model.gallery_category_id = Set(gallery_category_id);
                }
            }
            _ => {}
        }
    }

    // Save the updated Retreat
    let instance: RetreatGalleriesModel = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert to serializer
    let serializer: ReadRetreatGallerySerializer = instance.into();

    Ok(CustomResponse::<ReadRetreatGallerySerializer, ()>::builder(serializer).build())
}

async fn delete_retreat_gallery(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
    Path((retreat_id, gallery_id)): Path<(i64, i64)>,
) -> Result<Response<Body>, Response<Body>> {
    let instance: RetreatGalleriesModel = RetreatGalleriesEntity::find()
        .filter(RetreatGalleriesColumn::GalleryId.eq(gallery_id))
        .filter(RetreatGalleriesColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat gallery not found.", StatusCode::NOT_FOUND)
        })?;

    let image_relative_path: String = instance.image_path.clone();

    // Convert to ActiveModel for editing
    let active_model: RetreatGalleriesActiveModel = instance.into_active_model();

    remove_retreat_gallery(image_relative_path).await;

    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert model to serializer
    Ok(CustomResponse::<(), ()>::builder({})
        .message("Retreat gallery deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

async fn get_gallery_image(
    State(state): State<AppState>,
    Path((retreat_id, gallery_id)): Path<(i64, i64)>,
) -> Result<Response<Body>, Response<Body>> {
    let instance: RetreatGalleriesModel = RetreatGalleriesEntity::find()
        .filter(RetreatGalleriesColumn::GalleryId.eq(gallery_id))
        .filter(RetreatGalleriesColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat gallery not found.", StatusCode::NOT_FOUND)
        })?;

    let image_relative_path: String = instance.image_path.clone();
    println!("{}", image_relative_path);

    let result: Result<(Vec<u8>, HeaderMap), Box<dyn Error>> =
        read_retreat_gallery_with_headers(image_relative_path).await;
    let (bytes, headers) = match result {
        Ok(v) => v,
        Err(e) => {
            return Err(to_error_response_with_message(
                &e.to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let mut builder = Response::builder().status(StatusCode::OK);
    // Insert headers into the response
    {
        let headers_map = builder.headers_mut().unwrap();
        for (k, v) in headers.iter() {
            headers_map.insert(k, v.clone());
        }
    }

    let response = builder.body(Body::from(Bytes::from(bytes))).unwrap();
    Ok(response)
}

pub fn retreat_gallery_router() -> Router<AppState> {
    let router = Router::new()
        .route(
            "/retreats/{retreat_id}/galleries/",
            post(create_retreat_gallery),
        )
        .route(
            "/retreats/{retreat_id}/galleries/",
            get(list_retreat_gallery),
        )
        .route(
            "/retreats/{retreat_id}/galleries/{gallery_id}/",
            patch(update_retreat_gallery),
        )
        .route(
            "/retreats/{retreat_id}/galleries/{gallery_id}/",
            delete(delete_retreat_gallery),
        )
        .route(
            "/retreats/{retreat_id}/galleries/{gallery_id}/image/",
            get(get_gallery_image),
        );
    return router;
}
