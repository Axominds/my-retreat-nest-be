use std::any::Any;

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    entities_helper::{
        AdminUserColumn, AdminUserEntity, RetreatUserColumn, RetreatUserEntity, UserColumn,
        UserEntity, UserModel,
    },
    serializers::auth::TokenClaim,
    state::AppState,
    utils::jwt::get_access_token_claim,
};

async fn extract_authenticated_user<S>(
    parts: &mut Parts,
    state: &S,
) -> Result<(UserModel, TokenClaim), (StatusCode, String)>
where
    S: Send + Sync + std::fmt::Debug + Clone + 'static,
{
    let auth_header: &str = parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|value: &header::HeaderValue| value.to_str().ok())
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid Header".to_string(),
        ))?;

    let splitted_auth_header: Vec<&str> = auth_header.split(" ").collect();

    let (_schema, access_token) = (splitted_auth_header[0], splitted_auth_header[1]);

    let token_claim: TokenClaim = get_access_token_claim(access_token)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Token".to_string()))?;

    let email: String = token_claim.email.clone();
    let user_id: i64 = token_claim.user_id;
    let name: String = token_claim.name.clone();

    let state: AppState = (state as &dyn Any)
        .downcast_ref::<AppState>()
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to type cast app state".to_string(),
            )
        })
        .unwrap()
        .clone();

    let user: UserModel = UserEntity::find()
        .filter(UserColumn::Email.eq(email))
        .filter(UserColumn::UserId.eq(user_id))
        .filter(UserColumn::Name.eq(name))
        .one(&state.database)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    Ok((user, token_claim))
}

#[derive(Clone)]
pub struct AuthUser(pub UserModel);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync + std::fmt::Debug + Clone + 'static,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let (user, claims) = extract_authenticated_user(parts, state).await?;

        if claims.login_type != "normal" {
            return Err((StatusCode::FORBIDDEN, "User access required".to_string()));
        }

        Ok(AuthUser(user))
    }
}

#[derive(Clone)]
pub struct AuthAdmin(pub UserModel);

impl<S> FromRequestParts<S> for AuthAdmin
where
    S: Send + Sync + std::fmt::Debug + Clone + 'static,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let (user, claims) = extract_authenticated_user(parts, state).await?;

        if claims.login_type != "admin" {
            return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
        }

        let state: AppState = (state as &dyn Any)
            .downcast_ref::<AppState>()
            .ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to type cast app state".to_string(),
                )
            })
            .unwrap()
            .clone();

        AdminUserEntity::find()
            .filter(AdminUserColumn::UserId.eq(user.user_id))
            .one(&state.database)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::FORBIDDEN, "Admin access revoked".to_string()))?;

        Ok(AuthAdmin(user))
    }
}

#[derive(Clone)]
pub struct AuthRetreatUser(pub UserModel);

impl<S> FromRequestParts<S> for AuthRetreatUser
where
    S: Send + Sync + std::fmt::Debug + Clone + 'static,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let (user, claims) = extract_authenticated_user(parts, state).await?;

        if claims.login_type != "retreat" {
            return Err((
                StatusCode::FORBIDDEN,
                "Retreat user access required".to_string(),
            ));
        }

        let state: AppState = (state as &dyn Any)
            .downcast_ref::<AppState>()
            .ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to type cast app state".to_string(),
                )
            })
            .unwrap()
            .clone();

        RetreatUserEntity::find()
            .filter(RetreatUserColumn::UserId.eq(user.user_id))
            .one(&state.database)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| {
                (
                    StatusCode::FORBIDDEN,
                    "Retreat user access revoked".to_string(),
                )
            })?;

        Ok(AuthRetreatUser(user))
    }
}
