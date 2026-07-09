use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{Response, StatusCode},
    routing::post,
};
use chrono::{FixedOffset, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    entities_helper::{
        AdminUserColumn, AdminUserEntity, PasswordResetTokenActiveModel,
        PasswordResetTokenColumn, PasswordResetTokenEntity, RetreatUserColumn, RetreatUserEntity,
        UserColumn, UserEntity, UserModel,
    },
    serializers::auth::{
        ForgotPasswordSerializer, LoginResponseSerializer, LoginSerializer, RefreshSerializer,
        ResetPasswordSerializer, TokenClaim,
    },
    state::AppState,
    utils::{
        jwt::{generate_access_token, generate_refresh_token, get_refresh_token_claim},
        password::{check_password, create_password},
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

    match payload.login_type.as_str() {
        "normal" => {}
        "admin" => {
            AdminUserEntity::find()
                .filter(AdminUserColumn::UserId.eq(instance.user_id))
                .one(&state.database)
                .await
                .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
                .ok_or_else(|| {
                    to_error_response_with_message(
                        "Not an admin user.",
                        StatusCode::FORBIDDEN,
                    )
                })?;
        }
        "retreat" => {
            RetreatUserEntity::find()
                .filter(RetreatUserColumn::UserId.eq(instance.user_id))
                .one(&state.database)
                .await
                .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
                .ok_or_else(|| {
                    to_error_response_with_message(
                        "Not a retreat user.",
                        StatusCode::FORBIDDEN,
                    )
                })?;
        }
        _ => {
            return Err(to_error_response_with_message(
                "Invalid login type.",
                StatusCode::BAD_REQUEST,
            ));
        }
    }

    let token_claim: TokenClaim = TokenClaim {
        user_id: instance.user_id,
        name: instance.name,
        email: instance.email,
        login_type: payload.login_type,
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

    let claims: TokenClaim = get_refresh_token_claim(&refresh_token)
        .await
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

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

async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    // Always return success to avoid user enumeration
    let user = UserEntity::find()
        .filter(UserColumn::Email.eq(&payload.email))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    if let Some(user) = user {
        let token = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        let active_model = PasswordResetTokenActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user.user_id),
            token: Set(token.clone()),
            expires_at: Set(expires_at.into()),
            used_at: Set(None),
            created_at: Set(Utc::now().into()),
        };

        active_model
            .insert(&state.database)
            .await
            .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

        let mailer = state.mailer.clone();
        let email = payload.email.clone();
        tokio::spawn(async move {
            mailer.send_reset_email(&email, &token).await;
        });
    }

    Ok(CustomResponse::<(), ()>::builder({})
        .message("If an account with that email exists, a password reset link has been sent.")
        .status_code(StatusCode::OK)
        .build())
}

async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordSerializer>,
) -> Result<Response<Body>, Response<Body>> {
    payload
        .validate()
        .map_err(|e| to_error_response(e, StatusCode::BAD_REQUEST))?;

    let now: chrono::DateTime<FixedOffset> = Utc::now().into();

    let token_record = PasswordResetTokenEntity::find()
        .filter(PasswordResetTokenColumn::Token.eq(&payload.token))
        .filter(PasswordResetTokenColumn::UsedAt.is_null())
        .filter(PasswordResetTokenColumn::ExpiresAt.gt(now.clone()))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message(
                "Invalid or expired reset token.",
                StatusCode::BAD_REQUEST,
            )
        })?;

    let hashed_password = create_password(&payload.new_password)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let user = UserEntity::find()
        .filter(UserColumn::UserId.eq(token_record.user_id))
        .one(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| {
            to_error_response_with_message("User not found.", StatusCode::NOT_FOUND)
        })?;

    let mut user_active_model = user.into_active_model();
    user_active_model.password = Set(hashed_password);
    user_active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    let mut token_active_model = token_record.into_active_model();
    token_active_model.used_at = Set(Some(now.into()));
    token_active_model
        .update(&state.database)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(CustomResponse::<(), ()>::builder({})
        .message("Password has been reset successfully.")
        .status_code(StatusCode::OK)
        .build())
}

pub fn auth_router() -> Router<AppState> {
    let router = Router::new()
        .route("/auth/login/", post(login))
        .route("/auth/refresh/", post(refresh))
        .route("/auth/forgot-password/", post(forgot_password))
        .route("/auth/reset-password/", post(reset_password));
    return router;
}
