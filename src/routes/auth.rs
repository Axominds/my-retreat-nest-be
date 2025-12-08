use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{Response, StatusCode},
    routing::post,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use validator::Validate;

use crate::{
    entities_helper::{UserColumn, UserEntity, UserModel},
    serializers::auth::{LoginResponseSerializer, LoginSerializer, RefreshSerializer, TokenClaim},
    state::AppState,
    utils::{
        jwt::{generate_access_token, generate_refresh_token, get_refresh_token_claim},
        password::check_password,
        response::{to_error_response, to_error_response_with_message, CustomResponse},
    },
};

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;
    let instance: UserModel = UserEntity::find()
        .filter(UserColumn::Email.eq(payload.email))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("User not found.", StatusCode::NOT_FOUND))?;

    let password_matched: bool = check_password(&payload.password, &instance.password)
        .await
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    if !password_matched {
        return Ok(CustomResponse::<(), ()>::builder({})
            .message("Invalid Password!")
            .status_code(StatusCode::BAD_REQUEST)
            .build());
    }
    let token_claim: TokenClaim = TokenClaim {
        user_id: instance.user_id,
        name: instance.name,
        email: instance.email,
    };

    let access_token: String = generate_access_token(token_claim.clone())
        .await
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let refresh_token: String = generate_refresh_token(token_claim)
        .await
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let serializer: LoginResponseSerializer = LoginResponseSerializer {
        access_token: access_token,
        refresh_token: refresh_token,
    };

    Ok(CustomResponse::<LoginResponseSerializer, ()>::builder(serializer).build())
}

async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let refresh_token: String = payload.refresh_token;

    let claims: TokenClaim = get_refresh_token_claim(&refresh_token).await.map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let token_claim: TokenClaim = claims.clone();

    let email: String = claims.email;
    let user_id: i64 = claims.user_id;
    let name: String = claims.name;

    UserEntity::find()
        .filter(UserColumn::Email.eq(email))
        .filter(UserColumn::UserId.eq(user_id))
        .filter(UserColumn::Name.eq(name))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| to_error_response_with_message("User not found.", StatusCode::NOT_FOUND))?;

    let access_token: String = generate_access_token(token_claim.clone())
        .await
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let refresh_token: String = generate_refresh_token(token_claim)
        .await
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let serializer: LoginResponseSerializer = LoginResponseSerializer {
        access_token: access_token.clone(),
        refresh_token: refresh_token.clone(),
    };

    Ok(CustomResponse::<LoginResponseSerializer, ()>::builder(serializer).build())
}

pub fn auth_router() -> Router<AppState> {
    let router = Router::new()
        .route("/auth/login/", post(login))
        .route("/auth/refresh/", post(refresh));
    return router;
}
