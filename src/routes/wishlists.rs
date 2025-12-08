use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{Response, StatusCode},
    routing::{delete, get, post},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
};

use crate::{
    entities_helper::{
        RetreatColumn, RetreatEntity, WishlistActiveModel, WishlistColumn, WishlistEntity,
        WishlistModel,
    },
    serializers::wishlists::ReadWishlistSerializer,
    state::AppState,
    utils::{
        extractors::auth::AuthUser,
        response::{CustomResponse, to_error_response, to_error_response_with_message},
    },
};

async fn create_wishlist_item(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(retreat_id): Path<i64>,
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

    let wishlist = WishlistEntity::find()
        .filter(WishlistColumn::RetreatId.eq(retreat_id))
        .filter(WishlistColumn::UserId.eq(user.user_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    if wishlist.is_some() {
        return Ok(CustomResponse::<(), ()>::builder({})
            .message("Retreat added to wishlist successfully.")
            .status_code(StatusCode::CREATED)
            .build());
    }

    let active_model: WishlistActiveModel = WishlistActiveModel {
        retreat_id: Set(retreat_id),
        user_id: Set(user.user_id),
        ..Default::default()
    };

    // save wishlist
    active_model
        .save(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert model to serializer
    Ok(CustomResponse::<(), ()>::builder({})
        .message("Retreat added to wishlist successfully.")
        .status_code(StatusCode::CREATED)
        .build())
}

async fn delete_wishlist_item(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(retreat_id): Path<i64>,
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
    let active_model = WishlistEntity::find()
        .filter(WishlistColumn::RetreatId.eq(retreat_id))
        .filter(WishlistColumn::UserId.eq(user.user_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("Wishlist not found.", StatusCode::NOT_FOUND)
        })?
        .into_active_model();

    active_model
        .delete(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    // Convert model to serializer
    Ok(CustomResponse::<(), ()>::builder({})
        .message("Retreat deleted from wishlist successfully.")
        .status_code(StatusCode::NO_CONTENT)
        .build())
}

async fn list_wishlist_items(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<Response<Body>, Response<Body>> {
    // Convert model to serializer
    let instances: Vec<WishlistModel> = WishlistEntity::find()
        .filter(WishlistColumn::UserId.eq(user.user_id))
        .all(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    // Convert model to serializer
    let serializers: Vec<ReadWishlistSerializer> =
        instances.into_iter().map(|model| model.into()).collect();
    Ok(CustomResponse::<Vec<ReadWishlistSerializer>, ()>::builder(serializers).build())
}

pub fn wishlist_router() -> Router<AppState> {
    let router = Router::new()
        .route(
            "/users/wishlists/retreats/{retreat_id}/",
            post(create_wishlist_item),
        )
        .route(
            "/users/wishlists/retreats/{retreat_id}/",
            delete(delete_wishlist_item),
        )
        .route("/users/wishlists/retreats/", get(list_wishlist_items));
    return router;
}
