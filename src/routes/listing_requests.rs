use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{Response, StatusCode},
    routing::{get, patch, post},
};
use chrono::Utc;
use rand::Rng;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TryIntoModel,
};
use sea_orm::sea_query::{Expr, extension::postgres::PgExpr};
use validator::Validate;

use crate::{
    entities_helper::{
        ListingRequestActiveModel, ListingRequestColumn, ListingRequestEntity,
        ListingRequestModel, RetreatActiveModel, RetreatColumn, RetreatEntity,
        RetreatUserActiveModel, UserActiveModel, UserColumn, UserEntity, UserModel,
    },
    serializers::{
        listing_requests::{
            ApproveListingRequestSerializer, CreateListingRequestSerializer,
            ListingRequestFilter, ReadListingRequestSerializer,
            RejectListingRequestSerializer, UpdateListingRequestSerializer,
        },
        pagination::{Paginate, PaginationMeta},
        retreats::ReadRetreatSerializer,
    },
    set_active_model_fields,
    state::AppState,
    utils::{
        extractors::auth::AuthAdmin,
        password::create_password,
        response::{CustomResponse, to_error_response, to_error_response_with_message},
    },
};

fn generate_temp_password() -> String {
    let charset: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..12)
        .map(|_| {
            let idx = rng.gen_range(0..charset.len());
            charset[idx] as char
        })
        .collect()
}

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
) -> Result<String, Response<Body>> {
    let mut slug = base_slug.to_string();
    let mut counter = 0;
    loop {
        let exists = RetreatEntity::find()
            .filter(RetreatColumn::Slug.eq(&slug))
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

async fn create_listing_request(
    State(state): State<AppState>,
    Json(payload): Json<CreateListingRequestSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let active_model: ListingRequestActiveModel =
        set_active_model_fields!(payload, ListingRequestActiveModel, {
            owner_name,
            owner_email,
            owner_phone,
            retreat_name,
            retreat_description,
            category_id,
            retreat_email,
            retreat_phone,
            latitude,
            longitude,
            address,
            budget_min,
            budget_max,
            social_links,
        });

    let saved = active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let model: ListingRequestModel = saved
        .try_into_model()
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadListingRequestSerializer = model.into();

    Ok(CustomResponse::<ReadListingRequestSerializer, ()>::builder(serializer)
        .message("Your listing request has been submitted. We will review it shortly.")
        .status_code(StatusCode::CREATED)
        .build())
}

async fn list_listing_requests(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Query(filter): Query<ListingRequestFilter>,
) -> Result<Response<Body>, Response<Body>> {
    let mut query = ListingRequestEntity::find();

    if let Some(ref status) = filter.status {
        query = query.filter(ListingRequestColumn::Status.eq(status));
    }

    if let Some(ref search) = filter.search {
        query = query.filter(
            Expr::col(ListingRequestColumn::OwnerName)
                .ilike(format!("%{}%", search))
                .or(Expr::col(ListingRequestColumn::OwnerEmail)
                    .ilike(format!("%{}%", search)))
                .or(Expr::col(ListingRequestColumn::RetreatName)
                    .ilike(format!("%{}%", search))),
        );
    }

    match filter.sort_by.as_deref() {
        Some("oldest") => {
            query = query.order_by(ListingRequestColumn::ListingRequestId, Order::Asc);
        }
        _ => {
            query = query.order_by(ListingRequestColumn::ListingRequestId, Order::Desc);
        }
    }

    let total: u64 = query.clone().count(&state.database).await.unwrap();
    let instances: Vec<ListingRequestModel> = query
        .limit(filter.limit())
        .offset(filter.offset())
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializers: Vec<ReadListingRequestSerializer> =
        instances.into_iter().map(|m| m.into()).collect();

    let pagination_meta = filter.build_meta(total);

    Ok(CustomResponse::<Vec<ReadListingRequestSerializer>, PaginationMeta>::builder(
        serializers,
    )
    .meta(pagination_meta)
    .build())
}

async fn get_listing_request(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Path(request_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instance = ListingRequestEntity::find()
        .filter(ListingRequestColumn::ListingRequestId.eq(request_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Listing request not found.", StatusCode::NOT_FOUND)
        })?;

    let serializer: ReadListingRequestSerializer = instance.into();
    Ok(CustomResponse::<ReadListingRequestSerializer, ()>::builder(serializer).build())
}

async fn approve_listing_request(
    State(state): State<AppState>,
    AuthAdmin(admin): AuthAdmin,
    Path(request_id): Path<i64>,
    Json(payload): Json<ApproveListingRequestSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    let request = ListingRequestEntity::find()
        .filter(ListingRequestColumn::ListingRequestId.eq(request_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Listing request not found.", StatusCode::NOT_FOUND)
        })?;

    if request.status != "pending" {
        return Err(to_error_response_with_message(
            "Listing request has already been processed.",
            StatusCode::BAD_REQUEST,
        ));
    }

    let temp_password: String;
    let user_id: i64;

    let existing_user = UserEntity::find()
        .filter(UserColumn::Email.eq(&request.owner_email))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let is_new_user: bool;
    let user_email: String;

    if let Some(user) = existing_user {
        user_id = user.user_id;
        user_email = user.email.clone();
        is_new_user = false;
        temp_password = String::new();
    } else {
        temp_password = generate_temp_password();
        let hashed_password = create_password(&temp_password)
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

        let user_active_model = UserActiveModel {
            name: Set(request.owner_name.clone()),
            email: Set(request.owner_email.clone()),
            password: Set(hashed_password),
            phone: Set(request.owner_phone.clone()),
            ..Default::default()
        };

        let saved_user: UserModel = user_active_model
            .save(&state.database)
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
            .try_into_model()
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

        user_id = saved_user.user_id;
        user_email = saved_user.email;
        is_new_user = true;
    }

    let base_slug = payload.slug.unwrap_or_else(|| slugify(&request.retreat_name));
    let slug = ensure_unique_slug(&state.database, &base_slug).await?;

    let retreat_active_model = RetreatActiveModel {
        name: Set(request.retreat_name.clone()),
        description: Set(request.retreat_description.clone()),
        category_id: Set(request.category_id),
        slug: Set(slug),
        social_links: Set(request.social_links.clone()),
        email: Set(request.retreat_email.clone().unwrap_or_default()),
        phone: Set(request.retreat_phone.clone().unwrap_or_default()),
        latitude: Set(request.latitude.unwrap_or_default()),
        longitude: Set(request.longitude.unwrap_or_default()),
        address: Set(request.address.clone()),
        budget_min: Set(request.budget_min),
        budget_max: Set(request.budget_max),
        is_published: Set(true),
        created_by: Set(Some(admin.user_id)),
        updated_by: Set(Some(admin.user_id)),
        ..Default::default()
    };

    let saved_retreat = retreat_active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let retreat_model = saved_retreat
        .try_into_model()
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let retreat_user_active_model = RetreatUserActiveModel {
        retreat_id: Set(retreat_model.retreat_id),
        user_id: Set(user_id),
        role: Set(Some("owner".to_string())),
        created_by: Set(Some(admin.user_id)),
        ..Default::default()
    };

    retreat_user_active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let owner_name = request.owner_name.clone();
    let retreat_name = request.retreat_name.clone();

    let mut request_active_model = request.into_active_model();
    request_active_model.status = Set("approved".to_string());
    request_active_model.retreat_id = Set(Some(retreat_model.retreat_id));
    request_active_model.reviewed_by = Set(Some(admin.user_id));
    request_active_model.reviewed_at = Set(Some(Utc::now().into()));

    request_active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let mailer = state.mailer.clone();
    let to_email = user_email.clone();
    tokio::spawn(async move {
        mailer
            .send_listing_approved_email(
                &to_email,
                &owner_name,
                &retreat_name,
                if is_new_user {
                    Some(&temp_password)
                } else {
                    None
                },
            )
            .await;
    });

    let retreat_serializer: ReadRetreatSerializer = retreat_model.into();

    let msg = if is_new_user {
        "Listing approved. Retreat created. Email sent with credentials."
    } else {
        "Listing approved. Owner added to existing account."
    };

    Ok(CustomResponse::<ReadRetreatSerializer, ()>::builder(retreat_serializer)
        .message(msg)
        .status_code(StatusCode::OK)
        .build())
}

async fn reject_listing_request(
    State(state): State<AppState>,
    AuthAdmin(admin): AuthAdmin,
    Path(request_id): Path<i64>,
    Json(payload): Json<RejectListingRequestSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    let request = ListingRequestEntity::find()
        .filter(ListingRequestColumn::ListingRequestId.eq(request_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Listing request not found.", StatusCode::NOT_FOUND)
        })?;

    if request.status != "pending" {
        return Err(to_error_response_with_message(
            "Listing request has already been processed.",
            StatusCode::BAD_REQUEST,
        ));
    }

    let mut active_model = request.into_active_model();
    active_model.status = Set("rejected".to_string());
    active_model.reviewed_by = Set(Some(admin.user_id));
    active_model.reviewed_at = Set(Some(Utc::now().into()));
    active_model.rejection_reason = Set(payload.rejection_reason);

    active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(CustomResponse::<(), ()>::builder({})
        .message("Listing request rejected.")
        .status_code(StatusCode::OK)
        .build())
}

async fn update_listing_request(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Path(request_id): Path<i64>,
    Json(payload): Json<UpdateListingRequestSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    let request = ListingRequestEntity::find()
        .filter(ListingRequestColumn::ListingRequestId.eq(request_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Listing request not found.", StatusCode::NOT_FOUND)
        })?;

    if request.status != "pending" {
        return Err(to_error_response_with_message(
            "Only pending requests can be edited.",
            StatusCode::BAD_REQUEST,
        ));
    }

    let mut active_model = request.into_active_model();
    if let Some(v) = payload.owner_name { active_model.owner_name = Set(v); }
    if let Some(v) = payload.owner_email { active_model.owner_email = Set(v); }
    if let Some(v) = payload.owner_phone { active_model.owner_phone = Set(Some(v)); }
    if let Some(v) = payload.retreat_name { active_model.retreat_name = Set(v); }
    if let Some(v) = payload.retreat_description { active_model.retreat_description = Set(Some(v)); }
    if let Some(v) = payload.category_id { active_model.category_id = Set(v); }
    if let Some(v) = payload.retreat_email { active_model.retreat_email = Set(Some(v)); }
    if let Some(v) = payload.retreat_phone { active_model.retreat_phone = Set(Some(v)); }
    if let Some(v) = payload.latitude { active_model.latitude = Set(Some(v)); }
    if let Some(v) = payload.longitude { active_model.longitude = Set(Some(v)); }
    if let Some(v) = payload.address { active_model.address = Set(Some(v)); }
    if let Some(v) = payload.budget_min { active_model.budget_min = Set(Some(v)); }
    if let Some(v) = payload.budget_max { active_model.budget_max = Set(Some(v)); }
    if let Some(v) = payload.social_links { active_model.social_links = Set(v); }

    let saved = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let model: ListingRequestModel = saved
        .try_into_model()
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadListingRequestSerializer = model.into();

    Ok(CustomResponse::<ReadListingRequestSerializer, ()>::builder(serializer)
        .message("Listing request updated.")
        .build())
}

pub fn listing_request_router() -> Router<AppState> {
    Router::new()
        .route("/listing-requests/", post(create_listing_request))
        .route("/listing-requests/", get(list_listing_requests))
        .route("/listing-requests/{id}/", get(get_listing_request))
        .route("/listing-requests/{id}/", patch(update_listing_request))
        .route("/listing-requests/{id}/approve/", post(approve_listing_request))
        .route("/listing-requests/{id}/reject/", post(reject_listing_request))
}
