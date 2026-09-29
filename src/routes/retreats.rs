use axum::{
    Json, Router,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, Response, StatusCode},
    routing::{delete, get, patch, post},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, ExprTrait, IntoActiveModel,
    Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait, TryIntoModel,
};
use sea_orm::sea_query::{Expr, SimpleExpr, extension::postgres::PgExpr};
use std::borrow::Cow;
use serde::Deserialize;
use validator::Validate;

use crate::{
    entities_helper::{
        AmenityColumn, AmenityEntity, AmenityModel, RetreatActiveModel,
        RetreatAmenityActiveModel, RetreatAmenityColumn, RetreatAmenityEntity,
        RetreatColumn, RetreatEntity, RetreatModel, RetreatReviewColumn,
        RetreatReviewEntity, RetreatUserActiveModel, RetreatUserColumn, RetreatUserEntity,
        RetreatUserModel, UserActiveModel, UserColumn, UserEntity, UserModel,
    },
    env::ENV,
    serializers::{
        amenities::ReadAmenitySerializer,
        pagination::{Paginate, PaginationMeta},
        retreats::{
            CreateRetreatSerializer, CreateRetreatUserSerializer, ReadRetreatSerializer,
            ReadRetreatUserSerializer, RetreatFilter, UpdateRetreatSerializer,
            UpdateRetreatUserSerializer, ValidateRetreatSerializer,
        },
    },
    set_active_model_fields, set_fields,
    state::AppState,
    utils::{
        extractors::auth::{
            AuthAdmin, AuthAdminOrRetreatUser, ensure_retreat_membership, ensure_team_manager,
        },
        password::create_password,
        response::{CustomResponse, to_error_response, to_error_response_with_message},
        storage::{self, read_image_with_headers},
        tenant::{TenantResolveError, resolve_tenant_slug},
    },
};

/// Great-circle distance in kilometres between two WGS84 points.
fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    EARTH_RADIUS_KM * 2.0 * a.sqrt().asin()
}

async fn create_retreat(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Json(payload): Json<CreateRetreatSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let active_model: RetreatActiveModel = set_active_model_fields!(payload, RetreatActiveModel, {
        name,
        description,
        story,
        category_id,
        slug,
        social_links,
        email,
        phone,
        latitude,
        longitude,
        address
    });

    // save Retreat
    let active_model: RetreatActiveModel = active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    // convert to ReadRetreatSerializer serializer
    let saved_retreat: RetreatModel = active_model
        .try_into_model()
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    let mut serializer: ReadRetreatSerializer = saved_retreat.into();
    serializer.amenities = Vec::new();
    Ok(CustomResponse::<ReadRetreatSerializer, ()>::builder(serializer)
        .message("Retreat created successfully.")
        .status_code(StatusCode::CREATED)
        .build())
}

async fn list_retreats(
    State(state): State<AppState>,
    Query(filter): Query<RetreatFilter>,
) -> Result<Response<Body>, Response<Body>> {
    // Geo params: radius filter requires the full trio; sort_by=distance
    // and distance_km only need lat+lng.
    let geo_filter: Option<(f64, f64, f64)> =
        match (filter.latitude, filter.longitude, filter.radius_km) {
            (Some(lat), Some(lng), Some(radius)) => {
                if !(-90.0..=90.0).contains(&lat) {
                    return Err(to_error_response_with_message(
                        "Invalid latitude. Must be between -90 and 90.",
                        StatusCode::BAD_REQUEST,
                    ));
                }
                if !(-180.0..=180.0).contains(&lng) {
                    return Err(to_error_response_with_message(
                        "Invalid longitude. Must be between -180 and 180.",
                        StatusCode::BAD_REQUEST,
                    ));
                }
                if !(radius > 0.0 && radius <= 20000.0) {
                    return Err(to_error_response_with_message(
                        "Invalid radius_km. Must be greater than 0.",
                        StatusCode::BAD_REQUEST,
                    ));
                }
                Some((lat, lng, radius))
            }
            (None, None, None) => None,
            (Some(lat), Some(lng), None) => {
                if !(-90.0..=90.0).contains(&lat) {
                    return Err(to_error_response_with_message(
                        "Invalid latitude. Must be between -90 and 90.",
                        StatusCode::BAD_REQUEST,
                    ));
                }
                if !(-180.0..=180.0).contains(&lng) {
                    return Err(to_error_response_with_message(
                        "Invalid longitude. Must be between -180 and 180.",
                        StatusCode::BAD_REQUEST,
                    ));
                }
                None
            }
            (None, None, Some(_)) => {
                return Err(to_error_response_with_message(
                    "latitude and longitude must be provided together with radius_km.",
                    StatusCode::BAD_REQUEST,
                ));
            }
            _ => {
                return Err(to_error_response_with_message(
                    "latitude and longitude must be provided together.",
                    StatusCode::BAD_REQUEST,
                ));
            }
        };
    let geo_center: Option<(f64, f64)> = geo_filter
        .map(|(lat, lng, _)| (lat, lng))
        .or_else(|| match (filter.latitude, filter.longitude) {
            (Some(lat), Some(lng)) => Some((lat, lng)),
            _ => None,
        });

    let mut query = RetreatEntity::find();

    if let Some(val) = filter.is_published {
        query = query.filter(RetreatColumn::IsPublished.eq(val));
    }

    if let Some(ref search) = filter.search {
        query = query.filter(
            Expr::col(RetreatColumn::Name)
                .ilike(format!("%{}%", search))
                .or(Expr::col(RetreatColumn::Slug)
                    .ilike(format!("%{}%", search))),
        );
    }

    if let Some(category_id) = filter.category_id {
        query = query.filter(RetreatColumn::CategoryId.eq(category_id));
    }

    if let Some(min) = filter.budget_min {
        query = query.filter(RetreatColumn::BudgetMax.gte(min));
    }

    if let Some(max) = filter.budget_max {
        query = query.filter(RetreatColumn::BudgetMin.lte(max));
    }

    if let Some(rating) = filter.rating {
        query = query.filter(
            Expr::cust_with_values(
                "(SELECT AVG(rating) FROM retreat_reviews WHERE retreat_reviews.retreat_id = retreats.retreat_id) >= $1",
                vec![rating],
            )
        );
    }

    if let Some(val) = filter.is_featured {
        query = query.filter(RetreatColumn::IsFeatured.eq(val));
    }

    if let Some(ref amenity_ids_str) = filter.amenity_ids {
        let ids: Vec<i64> = amenity_ids_str
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect();
        if !ids.is_empty() {
            query = query.filter(Expr::cust_with_values(
                "retreats.retreat_id IN (
                    SELECT retreat_amenities.retreat_id
                    FROM retreat_amenities
                    WHERE retreat_amenities.amenity_id = ANY($1)
                    GROUP BY retreat_amenities.retreat_id
                    HAVING COUNT(DISTINCT retreat_amenities.amenity_id) = cardinality($1)
                )",
                vec![ids],
            ));
        }
    }

    if let Some((lat, lng, radius)) = geo_filter {
        query = query.filter(Expr::cust_with_values(
            "(6371 * acos(least(1.0, greatest(-1.0, \
                cos(radians($1)) * cos(radians(retreats.latitude::float8)) * \
                cos(radians(retreats.longitude::float8) - radians($2)) + \
                sin(radians($1)) * sin(radians(retreats.latitude::float8)))))) <= $3",
            vec![lat, lng, radius],
        ));
    }

    match filter.sort_by.as_deref() {
        Some("name") => {
            let order = match filter.sort_order.as_deref() {
                Some("desc") => Order::Desc,
                _ => Order::Asc,
            };
            query = query.order_by(RetreatColumn::Name, order);
        }
        Some("oldest") => {
            query = query.order_by(RetreatColumn::RetreatId, Order::Asc);
        }
        Some("status") => {
            query = query
                .order_by(RetreatColumn::IsPublished, Order::Desc)
                .order_by(RetreatColumn::Name, Order::Asc);
        }
        Some("rating") => {
            query = query.order_by(
                SimpleExpr::Custom(Cow::Borrowed(
                    "COALESCE((SELECT AVG(rating) FROM retreat_reviews WHERE retreat_id = retreats.retreat_id), 0)",
                )),
                Order::Desc,
            );
        }
        Some("distance") => {
            match geo_center {
                Some((lat, lng)) => {
                    query = query.order_by(
                        Expr::cust_with_values(
                            "(6371 * acos(least(1.0, greatest(-1.0, \
                                cos(radians($1)) * cos(radians(retreats.latitude::float8)) * \
                                cos(radians(retreats.longitude::float8) - radians($2)) + \
                                sin(radians($1)) * sin(radians(retreats.latitude::float8))))))",
                            vec![lat, lng],
                        ),
                        Order::Asc,
                    );
                }
                None => {
                    return Err(to_error_response_with_message(
                        "sort_by=distance requires latitude and longitude.",
                        StatusCode::BAD_REQUEST,
                    ));
                }
            }
        }
        _ => {
            // Default to closest-first when a radius search is active.
            if let Some((lat, lng, _)) = geo_filter {
                query = query.order_by(
                    Expr::cust_with_values(
                        "(6371 * acos(least(1.0, greatest(-1.0, \
                            cos(radians($1)) * cos(radians(retreats.latitude::float8)) * \
                            cos(radians(retreats.longitude::float8) - radians($2)) + \
                            sin(radians($1)) * sin(radians(retreats.latitude::float8))))))",
                        vec![lat, lng],
                    ),
                    Order::Asc,
                );
            } else {
                query = query.order_by(RetreatColumn::RetreatId, Order::Desc);
            }
        }
    }

    let instances: Vec<RetreatModel> = query.clone()
        .limit(filter.limit())
        .offset(filter.offset())
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let retreat_ids: Vec<i64> = instances.iter().map(|m| m.retreat_id).collect();
    let distances: Option<Vec<f64>> = geo_center.map(|(lat, lng)| {
        instances
            .iter()
            .map(|m| {
                let rlat: f64 = m.latitude.to_string().parse().unwrap_or(0.0);
                let rlng: f64 = m.longitude.to_string().parse().unwrap_or(0.0);
                haversine_km(lat, lng, rlat, rlng)
            })
            .collect()
    });
    let mut serializers: Vec<ReadRetreatSerializer> =
        instances.into_iter().map(|model| model.into()).collect();
    if let Some(ds) = distances {
        for (serializer, d) in serializers.iter_mut().zip(ds) {
            serializer.distance_km = Some(d);
        }
    }

    let reviews = RetreatReviewEntity::find()
        .filter(RetreatReviewColumn::RetreatId.is_in(retreat_ids.clone()))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let mut sum_map: std::collections::HashMap<i64, f64> = std::collections::HashMap::new();
    let mut count_map: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
    for review in &reviews {
        *sum_map.entry(review.retreat_id).or_default() += review.rating;
        *count_map.entry(review.retreat_id).or_default() += 1;
    }
    let avg_map: std::collections::HashMap<i64, f64> = sum_map
        .into_iter()
        .filter_map(|(id, sum)| count_map.get(&id).map(|&count| (id, sum / count as f64)))
        .collect();
    for (serializer, id) in serializers.iter_mut().zip(&retreat_ids) {
        serializer.average_rating = avg_map.get(id).copied();
    }

    for (serializer, id) in serializers.iter_mut().zip(&retreat_ids) {
        serializer.amenities = load_amenities_for_retreat(&state, *id).await?;
    }

    let total: u64 = query.count(&state.database).await.unwrap();
    let pagination_meta = filter.build_meta(total);
    Ok(CustomResponse::<Vec<ReadRetreatSerializer>, PaginationMeta>::builder(serializers).meta(pagination_meta).build())
}

async fn get_retreat(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
    Query(filter): Query<RetreatFilter>,
) -> Result<Response<Body>, Response<Body>> {
    let mut query = RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id));
    if filter.is_published == Some(true) {
        query = query.filter(RetreatColumn::IsPublished.eq(true));
    }

    let instance = query
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    let retreat_id = instance.retreat_id;
    let mut serializer: ReadRetreatSerializer = instance.into();

    let reviews = RetreatReviewEntity::find()
        .filter(RetreatReviewColumn::RetreatId.eq(retreat_id))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    if !reviews.is_empty() {
        let sum: f64 = reviews.iter().map(|r| r.rating).sum();
        serializer.average_rating = Some(sum / reviews.len() as f64);
    }

    serializer.amenities = load_amenities_for_retreat(&state, retreat_id).await?;

    Ok(CustomResponse::<ReadRetreatSerializer, ()>::builder(serializer).build())
}

async fn update_retreat(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path(retreat_id): Path<i64>,
    Json(payload): Json<UpdateRetreatSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;
    // Tenant scope: retreat staff may only edit their own retreat.
    // Global admins bypass.
    ensure_retreat_membership(&state.database, &principal, retreat_id).await?;
    // Find existing Retreat
    let instance = RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    // Convert to ActiveModel for editing
    let mut active_model: RetreatActiveModel = instance.into_active_model();

    set_fields!(
        active_model,
        payload,
        name,
        description,
        story,
        category_id,
        slug,
        social_links,
        email,
        phone,
        longitude,
        latitude,
        address,
        budget_min,
        budget_max,
        is_published,
        is_featured
    );

    // Save the updated Retreat
    let instance = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert to serializer
    let updated_instance: RetreatModel = instance;
    let retreat_id = updated_instance.retreat_id;
    let mut serializer: ReadRetreatSerializer = updated_instance.into();
    serializer.amenities = load_amenities_for_retreat(&state, retreat_id).await?;

    // Return success
    Ok(CustomResponse::<ReadRetreatSerializer, ()>::builder(serializer)
        .message("Retreat updated successfully.")
        .status_code(StatusCode::OK)
        .build())
}

async fn delete_retreat(
    State(state): State<AppState>,
    AuthAdmin(_): AuthAdmin,
    Path(retreat_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    // Query a single record
    let instance = RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    // Convert to ActiveModel for editing
    let active_model: RetreatActiveModel = instance.into_active_model();

    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert model to serializer
    Ok(CustomResponse::<(), ()>::builder({})
        .message("Retreat deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

async fn create_retreat_user(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path(retreat_id): Path<i64>,
    Json(payload): Json<CreateRetreatUserSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    // Ensure retreat exists
    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    // Tenant scope: only members of this retreat (owner/manager) may invite.
    // Global admins bypass.
    let membership =
        ensure_retreat_membership(&state.database, &principal, retreat_id).await?;
    ensure_team_manager(membership.as_ref())?;

    // Check if user exists
    let user = UserEntity::find()
        .filter(UserColumn::Email.eq(&payload.email))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let user_id: i64 = if let Some(user) = user {
        if user.name != payload.name {
            // Early return: user exists with different name
            return Ok(CustomResponse::<(), ()>::builder({})
                .message(&format!(
                    "User exists with a different name <strong>{}</strong>.",
                    user.name
                ))
                .status_code(StatusCode::ACCEPTED)
                .build());
        }
        user.user_id
    } else {
        // Create new user
        let hashed_password = create_password("tempPassword")
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

        let user_active_model = UserActiveModel {
            name: Set(payload.name),
            email: Set(payload.email),
            password: Set(hashed_password),
            ..Default::default()
        };

        let saved_user: UserModel = user_active_model
            .save(&state.database)
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
            .try_into_model()
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

        saved_user.user_id
    };

    // Associate user with retreat
    let active_model: RetreatUserActiveModel = RetreatUserActiveModel {
        retreat_id: Set(retreat_id),
        user_id: Set(user_id),
        role: Set(Some(payload.role)),
        ..Default::default()
    };

    active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(CustomResponse::<(), ()>::builder({})
        .message("Staff added successfully.")
        .status_code(StatusCode::CREATED)
        .build())
}

async fn update_retreat_user(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path((retreat_id, retreat_user_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateRetreatUserSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;
    // Ensure retreat exists
    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    // Tenant scope: only members of this retreat (owner/manager) may change roles.
    // Global admins bypass.
    let membership =
        ensure_retreat_membership(&state.database, &principal, retreat_id).await?;
    ensure_team_manager(membership.as_ref())?;

    // Ensure the staff member belongs to the retreat
    let instance: RetreatUserModel = RetreatUserEntity::find()
        .filter(RetreatUserColumn::RetreatUserId.eq(retreat_user_id))
        .filter(RetreatUserColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("Staff not found.", StatusCode::NOT_FOUND))?;

    // Convert to ActiveModel for editing
    let mut active_model: RetreatUserActiveModel = instance.into_active_model();

    set_fields!(active_model, payload, role);

    active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(CustomResponse::<(), ()>::builder({})
        .message("Staff updated successfully.")
        .status_code(StatusCode::OK)
        .build())
}

async fn delete_retreat_user(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path((retreat_id, retreat_user_id)): Path<(i64, i64)>,
) -> Result<Response<Body>, Response<Body>> {
    // Ensure retreat exists
    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    // Tenant scope: only members of this retreat (owner/manager) may remove staff.
    // Global admins bypass.
    let membership =
        ensure_retreat_membership(&state.database, &principal, retreat_id).await?;
    ensure_team_manager(membership.as_ref())?;

    // Ensure retreat exists
    let instance: RetreatUserModel = RetreatUserEntity::find()
        .filter(RetreatUserColumn::RetreatUserId.eq(retreat_user_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("Staff not found.", StatusCode::NOT_FOUND))?;

    // Convert to ActiveModel for editing
    let active_model: RetreatUserActiveModel = instance.into_active_model();

    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert model to serializer
    Ok(CustomResponse::<(), ()>::builder({})
        .message("Staff deleted successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

async fn list_retreat_users(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path(retreat_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    // Tenant scope: only members of this retreat may view the team.
    // Global admins bypass.
    ensure_retreat_membership(&state.database, &principal, retreat_id).await?;
    let retreat_users: Vec<RetreatUserModel> = RetreatUserEntity::find()
        .filter(RetreatUserColumn::RetreatId.eq(retreat_id))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let user_ids: Vec<i64> = retreat_users.iter().map(|ru| ru.user_id).collect();

    let users: Vec<UserModel> = UserEntity::find()
        .filter(UserColumn::UserId.is_in(user_ids))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let user_map: std::collections::HashMap<i64, &UserModel> =
        users.iter().map(|u| (u.user_id, u)).collect();

    let serializers: Vec<ReadRetreatUserSerializer> = retreat_users
        .into_iter()
        .map(|ru| {
            let user = user_map.get(&ru.user_id).expect("User should exist");
            ReadRetreatUserSerializer {
                retreat_user_id: ru.retreat_user_id,
                retreat_id: ru.retreat_id,
                user_id: ru.user_id,
                name: user.name.clone(),
                email: user.email.clone(),
                role: ru.role,
            }
        })
        .collect();

    Ok(CustomResponse::<Vec<ReadRetreatUserSerializer>, ()>::builder(serializers).build())
}

async fn upload_retreat_thumbnail(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path(retreat_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Response<Body>, Response<Body>> {
    // Tenant scope: retreat staff may only manage their own retreat's images.
    // Global admins bypass.
    ensure_retreat_membership(&state.database, &principal, retreat_id).await?;
    let instance = RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
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
                    "retreat/thumbnail",
                    instance.thumbnail_image.clone(),
                )
                .await,
            );
        }
    }

    let image_path = image_path.ok_or_else(|| {
        to_error_response_with_message("Image file is required.", StatusCode::BAD_REQUEST)
    })?;

    let mut active_model: RetreatActiveModel = instance.into_active_model();
    active_model.thumbnail_image = Set(Some(image_path));
    active_model.updated_by = Set(Some(principal.user_id()));

    let instance = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadRetreatSerializer = instance.into();
    Ok(CustomResponse::<ReadRetreatSerializer, ()>::builder(serializer)
        .message("Thumbnail uploaded successfully.")
        .build())
}

async fn upload_retreat_banner(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path(retreat_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Response<Body>, Response<Body>> {
    // Tenant scope: retreat staff may only manage their own retreat's images.
    // Global admins bypass.
    ensure_retreat_membership(&state.database, &principal, retreat_id).await?;
    let instance = RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
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
                    "retreat/banner",
                    instance.banner_image.clone(),
                )
                .await,
            );
        }
    }

    let image_path = image_path.ok_or_else(|| {
        to_error_response_with_message("Image file is required.", StatusCode::BAD_REQUEST)
    })?;

    let mut active_model: RetreatActiveModel = instance.into_active_model();
    active_model.banner_image = Set(Some(image_path));
    active_model.updated_by = Set(Some(principal.user_id()));

    let instance = active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let serializer: ReadRetreatSerializer = instance.into();
    Ok(CustomResponse::<ReadRetreatSerializer, ()>::builder(serializer)
        .message("Banner uploaded successfully.")
        .build())
}

async fn get_retreat_thumbnail_image(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instance = RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
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

async fn get_retreat_banner_image(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let instance = RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    let image_path = instance.banner_image.ok_or_else(|| {
        to_error_response_with_message("Banner not found.", StatusCode::NOT_FOUND)
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

async fn load_amenities_for_retreat(
    state: &AppState,
    retreat_id: i64,
) -> Result<Vec<ReadAmenitySerializer>, Response<Body>> {
    let joins = RetreatAmenityEntity::find()
        .filter(RetreatAmenityColumn::RetreatId.eq(retreat_id))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let amenity_ids: Vec<i64> = joins.iter().map(|j| j.amenity_id).collect();
    if amenity_ids.is_empty() {
        return Ok(Vec::new());
    }

    let amenities: Vec<AmenityModel> = AmenityEntity::find()
        .filter(AmenityColumn::AmenityId.is_in(amenity_ids))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(amenities.into_iter().map(|a| a.into()).collect())
}

#[derive(Deserialize)]
struct SetRetreatAmenitiesSerializer {
    amenity_ids: Vec<i64>,
}

async fn list_retreat_amenities(
    State(state): State<AppState>,
    Path(retreat_id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let amenities = load_amenities_for_retreat(&state, retreat_id).await?;
    Ok(CustomResponse::<Vec<ReadAmenitySerializer>, ()>::builder(amenities).build())
}

async fn set_retreat_amenities(
    State(state): State<AppState>,
    AuthAdminOrRetreatUser(principal): AuthAdminOrRetreatUser,
    Path(retreat_id): Path<i64>,
    Json(payload): Json<SetRetreatAmenitiesSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    RetreatEntity::find()
        .filter(RetreatColumn::RetreatId.eq(retreat_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    // Tenant scope: retreat staff may only manage their own retreat's amenities.
    // Global admins bypass. (Previously any retreat user could edit any retreat.)
    ensure_retreat_membership(&state.database, &principal, retreat_id).await?;

    let txn = state
        .database
        .begin()
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    RetreatAmenityEntity::delete_many()
        .filter(RetreatAmenityColumn::RetreatId.eq(retreat_id))
        .exec(&txn)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    for amenity_id in &payload.amenity_ids {
        let active_model: RetreatAmenityActiveModel = RetreatAmenityActiveModel {
            retreat_id: Set(retreat_id),
            amenity_id: Set(*amenity_id),
            ..Default::default()
        };
        active_model
            .insert(&txn)
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    }

    txn.commit()
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let amenities = load_amenities_for_retreat(&state, retreat_id).await?;
    Ok(CustomResponse::<Vec<ReadAmenitySerializer>, ()>::builder(amenities)
        .message("Retreat amenities updated successfully.")
        .status_code(StatusCode::OK)
        .build())
}

async fn validate_retreat(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response<Body>, Response<Body>> {
    let slug: String = match resolve_tenant_slug(&headers, &ENV.root_domain) {
        Ok(slug) => slug,
        Err(TenantResolveError::NonTenant) => {
            return Err(to_error_response_with_message(
                "Retreat not found.",
                StatusCode::NOT_FOUND,
            ));
        }
        Err(TenantResolveError::Missing) => {
            return Err(to_error_response_with_message(
                "Missing tenant host information.",
                StatusCode::BAD_REQUEST,
            ));
        }
        Err(TenantResolveError::Invalid) => {
            return Err(to_error_response_with_message(
                "Invalid tenant host.",
                StatusCode::BAD_REQUEST,
            ));
        }
    };

    let instance = RetreatEntity::find()
        .filter(RetreatColumn::Slug.eq(slug))
        .filter(RetreatColumn::IsPublished.eq(true))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Retreat not found.", StatusCode::NOT_FOUND)
        })?;

    let serializer: ValidateRetreatSerializer = instance.into();
    Ok(CustomResponse::<ValidateRetreatSerializer, ()>::builder(serializer).build())
}

pub fn retreat_router() -> Router<AppState> {
    let router = Router::new()
        .route("/retreats/validate/", get(validate_retreat))
        .route("/retreats/", post(create_retreat))
        .route("/retreats/", get(list_retreats))
        .route("/retreats/{retreat_id}/", get(get_retreat))
        .route("/retreats/{retreat_id}/", patch(update_retreat))
        .route("/retreats/{retreat_id}/", delete(delete_retreat))
        .route("/retreats/{retreat_id}/users/", get(list_retreat_users).post(create_retreat_user))
        .route(
            "/retreats/{retreat_id}/users/{retreat_user_id}/",
            patch(update_retreat_user),
        )
        .route(
            "/retreats/{retreat_id}/users/{retreat_user_id}/",
            delete(delete_retreat_user),
        )
        .route("/retreats/{retreat_id}/amenities/", get(list_retreat_amenities).put(set_retreat_amenities))
        .route("/retreats/{retreat_id}/thumbnail/", post(upload_retreat_thumbnail))
        .route("/retreats/{retreat_id}/thumbnail/image/", get(get_retreat_thumbnail_image))
        .route("/retreats/{retreat_id}/banner/", post(upload_retreat_banner))
        .route("/retreats/{retreat_id}/banner/image/", get(get_retreat_banner_image));
    return router;
}

#[cfg(test)]
mod tests {
    use super::haversine_km;

    #[test]
    fn same_point_is_zero() {
        assert!(haversine_km(27.7172, 85.3240, 27.7172, 85.3240) < 1e-6);
    }

    #[test]
    fn kathmandu_to_pokhara_is_about_200km() {
        // Kathmandu (27.7172, 85.3240) -> Pokhara (28.2096, 83.9856)
        let d = haversine_km(27.7172, 85.3240, 28.2096, 83.9856);
        assert!((135.0..150.0).contains(&d), "got {d}");
    }

    #[test]
    fn antipodal_points_are_half_earth_circumference() {
        let d = haversine_km(0.0, 0.0, 0.0, 180.0);
        assert!((20000.0..20040.0).contains(&d), "got {d}");
    }
}
