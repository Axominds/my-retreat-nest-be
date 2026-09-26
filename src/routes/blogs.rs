use axum::{
    Json, Router,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{Response, StatusCode},
    routing::{delete, get, patch, post},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, ExprTrait, IntoActiveModel,
    Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TryIntoModel,
};
use sea_orm::sea_query::{Expr, extension::postgres::PgExpr};
use validator::Validate;

use crate::{
    entities_helper::{BlogActiveModel, BlogColumn, BlogEntity, BlogModel},
    serializers::{
        blogs::{BlogFilter, CreateBlogSerializer, ReadBlogSerializer, UpdateBlogSerializer},
        pagination::{Paginate, PaginationMeta},
    },
    set_active_model_fields, set_fields,
    state::AppState,
    utils::{
        extractors::auth::AuthAdmin,
        response::{CustomResponse, to_error_response, to_error_response_with_message},
        storage::{self, read_image_with_headers},
    },
};

fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .map(|c| if c == ' ' { '-' } else { c })
        .collect::<String>()
}

async fn ensure_unique_slug(
    db: &sea_orm::DatabaseConnection,
    base_slug: &str,
    exclude_blog_id: Option<i64>,
) -> Result<String, Response<Body>> {
    let mut slug = base_slug.to_string();
    let mut counter = 0;
    loop {
        let mut query = BlogEntity::find().filter(BlogColumn::Slug.eq(&slug));
        if let Some(exclude_id) = exclude_blog_id {
            query = query.filter(BlogColumn::BlogId.ne(exclude_id));
        }
        let exists = query
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

async fn create_blog(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Json(payload): Json<CreateBlogSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let base_slug = match &payload.slug {
        Some(s) if !s.trim().is_empty() => slugify(s),
        _ => slugify(&payload.title),
    };
    if base_slug.is_empty() {
        return Err(to_error_response_with_message(
            "Could not generate a slug from the title. Please provide a slug.",
            StatusCode::BAD_REQUEST,
        ));
    }
    let slug = ensure_unique_slug(&state.database, &base_slug, None).await?;

    let mut active_model: BlogActiveModel =
        set_active_model_fields!(payload, BlogActiveModel, {
            title,
            excerpt,
            content,
            tags
        });
    active_model.slug = Set(slug);
    if payload.is_published == Some(true) {
        active_model.is_published = Set(true);
    }

    let active_model: BlogActiveModel = active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    let serializer: ReadBlogSerializer = active_model
        .try_into_model()
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .into();
    Ok(CustomResponse::<ReadBlogSerializer, ()>::builder(serializer)
        .message("Blog created successfully.")
        .status_code(StatusCode::CREATED)
        .build())
}

async fn list_blogs(
    State(state): State<AppState>,
    Query(filter): Query<BlogFilter>,
) -> Result<Response<Body>, Response<Body>> {
    let mut query = BlogEntity::find();

    if let Some(val) = filter.is_published {
        query = query.filter(BlogColumn::IsPublished.eq(val));
    }

    if let Some(ref search) = filter.search {
        query = query.filter(
            Expr::col(BlogColumn::Title)
                .ilike(format!("%{}%", search))
                .or(Expr::col(BlogColumn::Slug).ilike(format!("%{}%", search)))
                .or(Expr::col(BlogColumn::Excerpt).ilike(format!("%{}%", search))),
        );
    }

    if let Some(ref tag) = filter.tag {
        query = query.filter(Expr::col(BlogColumn::Tags).ilike(format!("%{}%", tag)));
    }

    match filter.sort_by.as_deref() {
        Some("oldest") => {
            query = query.order_by(BlogColumn::CreatedAt, Order::Asc);
        }
        Some("title") => {
            query = query.order_by(BlogColumn::Title, Order::Asc);
        }
        _ => {
            query = query.order_by(BlogColumn::CreatedAt, Order::Desc);
        }
    }

    let total: u64 = query.clone().count(&state.database).await.unwrap();

    let instances: Vec<BlogModel> = query
        .limit(filter.limit())
        .offset(filter.offset())
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializers: Vec<ReadBlogSerializer> =
        instances.into_iter().map(|model| model.into()).collect();

    let pagination_meta = filter.build_meta(total);
    Ok(CustomResponse::<Vec<ReadBlogSerializer>, PaginationMeta>::builder(serializers)
        .meta(pagination_meta)
        .build())
}

async fn list_blog_tags(
    State(state): State<AppState>,
) -> Result<Response<Body>, Response<Body>> {
    let instances: Vec<BlogModel> = BlogEntity::find()
        .filter(BlogColumn::IsPublished.eq(true))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let mut tags: Vec<String> = Vec::new();
    for instance in &instances {
        if let Some(ref raw) = instance.tags {
            for part in raw.split(',') {
                let tag = part.trim().to_string();
                if !tag.is_empty() && !tags.iter().any(|t| t.eq_ignore_ascii_case(&tag)) {
                    tags.push(tag);
                }
            }
        }
    }
    tags.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

    Ok(CustomResponse::<Vec<String>, ()>::builder(tags).build())
}

async fn get_blog(
    State(state): State<AppState>,
    Path(blog_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instance = BlogEntity::find()
        .filter(BlogColumn::BlogId.eq(blog_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Blog not found.", StatusCode::NOT_FOUND)
        })?;

    let serializer: ReadBlogSerializer = instance.into();
    Ok(CustomResponse::<ReadBlogSerializer, ()>::builder(serializer).build())
}

async fn get_blog_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Response<Body>, Response<Body>> {
    let instance = BlogEntity::find()
        .filter(BlogColumn::Slug.eq(slug))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Blog not found.", StatusCode::NOT_FOUND)
        })?;

    let serializer: ReadBlogSerializer = instance.into();
    Ok(CustomResponse::<ReadBlogSerializer, ()>::builder(serializer).build())
}

async fn update_blog(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Path(blog_id): Path<i64>,
    Json(payload): Json<UpdateBlogSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;
    let instance = BlogEntity::find()
        .filter(BlogColumn::BlogId.eq(blog_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Blog not found.", StatusCode::NOT_FOUND)
        })?;

    let current_slug = instance.slug.clone();

    let mut active_model: BlogActiveModel = instance.into_active_model();

    if let Some(ref new_slug) = payload.slug {
        let base = slugify(new_slug);
        if base.is_empty() {
            return Err(to_error_response_with_message(
                "Slug must contain at least one alphanumeric character.",
                StatusCode::BAD_REQUEST,
            ));
        }
        if base != current_slug {
            let unique = ensure_unique_slug(&state.database, &base, Some(blog_id)).await?;
            active_model.slug = Set(unique);
        }
    }

    set_fields!(
        active_model,
        payload,
        title,
        excerpt,
        content,
        tags,
        is_published
    );

    let instance = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadBlogSerializer = instance.into();
    Ok(CustomResponse::<ReadBlogSerializer, ()>::builder(serializer)
        .message("Blog updated successfully.")
        .status_code(StatusCode::OK)
        .build())
}

async fn delete_blog(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Path(blog_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instance = BlogEntity::find()
        .filter(BlogColumn::BlogId.eq(blog_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Blog not found.", StatusCode::NOT_FOUND)
        })?;

    let active_model: BlogActiveModel = instance.into_active_model();

    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(CustomResponse::<(), ()>::builder({})
        .message("Blog deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

async fn upload_blog_cover(
    State(state): State<AppState>,
    AuthAdmin(user): AuthAdmin,
    Path(blog_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Response<Body>, Response<Body>> {
    let instance = BlogEntity::find()
        .filter(BlogColumn::BlogId.eq(blog_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Blog not found.", StatusCode::NOT_FOUND)
        })?;

    let mut image_path: Option<String> = None;
    loop {
        let field = multipart.next_field().await.map_err(|e| {
            eprintln!("Failed to read blog cover multipart field: {e}");
            to_error_response_with_message(
                "Failed to read uploaded file.",
                StatusCode::BAD_REQUEST,
            )
        })?;
        let Some(field) = field else { break };
        if field.name().unwrap_or("") != "image" {
            continue;
        }
        let file_name = field.file_name().ok_or_else(|| {
            to_error_response_with_message(
                "Uploaded file must have a filename.",
                StatusCode::BAD_REQUEST,
            )
        })?;
        let file_name = file_name.to_string();
        let file_content = field.bytes().await.map_err(|e| {
            eprintln!("Failed to read blog cover file bytes: {e}");
            to_error_response_with_message(
                "Failed to read uploaded file.",
                StatusCode::BAD_REQUEST,
            )
        })?;
        image_path = Some(
            storage::store_image(
                file_content,
                file_name,
                "blog/cover",
                instance.cover_image.clone(),
            )
            .await,
        );
    }

    let image_path = image_path.ok_or_else(|| {
        to_error_response_with_message("Image file is required.", StatusCode::BAD_REQUEST)
    })?;

    let mut active_model: BlogActiveModel = instance.into_active_model();
    active_model.cover_image = Set(Some(image_path));
    active_model.updated_by = Set(Some(user.user_id));

    let instance = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadBlogSerializer = instance.into();
    Ok(CustomResponse::<ReadBlogSerializer, ()>::builder(serializer)
        .message("Cover uploaded successfully.")
        .build())
}

async fn delete_blog_cover(
    State(state): State<AppState>,
    AuthAdmin(user): AuthAdmin,
    Path(blog_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instance = BlogEntity::find()
        .filter(BlogColumn::BlogId.eq(blog_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Blog not found.", StatusCode::NOT_FOUND)
        })?;

    let cover = instance.cover_image.clone().ok_or_else(|| {
        to_error_response_with_message("Cover not found.", StatusCode::NOT_FOUND)
    })?;

    let mut active_model: BlogActiveModel = instance.into_active_model();
    active_model.cover_image = Set(None);
    active_model.updated_by = Set(Some(user.user_id));

    let instance = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    storage::remove_image(cover).await;

    let serializer: ReadBlogSerializer = instance.into();
    Ok(CustomResponse::<ReadBlogSerializer, ()>::builder(serializer)
        .message("Cover deleted successfully.")
        .build())
}

async fn get_blog_cover_image(
    State(state): State<AppState>,
    Path(blog_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instance = BlogEntity::find()
        .filter(BlogColumn::BlogId.eq(blog_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Blog not found.", StatusCode::NOT_FOUND)
        })?;

    let image_path = instance.cover_image.ok_or_else(|| {
        to_error_response_with_message("Cover not found.", StatusCode::NOT_FOUND)
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

pub fn blog_router() -> Router<AppState> {
    let router = Router::new()
        .route("/blogs/", post(create_blog))
        .route("/blogs/", get(list_blogs))
        .route("/blogs/tags/", get(list_blog_tags))
        .route("/blogs/slug/{slug}/", get(get_blog_by_slug))
        .route("/blogs/{blog_id}/", get(get_blog))
        .route("/blogs/{blog_id}/", patch(update_blog))
        .route("/blogs/{blog_id}/", delete(delete_blog))
        .route("/blogs/{blog_id}/cover/", post(upload_blog_cover))
        .route("/blogs/{blog_id}/cover/", delete(delete_blog_cover))
        .route("/blogs/{blog_id}/cover/image/", get(get_blog_cover_image));
    return router;
}
