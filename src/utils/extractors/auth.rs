use std::any::Any;

use axum::{
    body::Body,
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
    response::Response,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entities_helper::{
        AdminUserColumn, AdminUserEntity, RetreatUserColumn, RetreatUserEntity, RetreatUserModel,
        UserColumn, UserEntity, UserModel,
    },
    serializers::auth::TokenClaim,
    state::AppState,
    utils::{
        jwt::get_access_token_claim,
        response::{to_error_response, to_error_response_with_message},
    },
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
            StatusCode::UNAUTHORIZED,
            "Missing or invalid Authorization header".to_string(),
        ))?;

    let access_token: &str = auth_header
        .strip_prefix("Bearer ")
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Missing or invalid Authorization header".to_string(),
        ))?;

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
pub struct OptionalAuthUser(pub Option<UserModel>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync + std::fmt::Debug + Clone + 'static,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match AuthUser::from_request_parts(parts, state).await {
            Ok(AuthUser(user)) => Ok(OptionalAuthUser(Some(user))),
            Err(_) => Ok(OptionalAuthUser(None)),
        }
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
pub enum AuthPrincipal {
    User(UserModel),
    Admin(UserModel),
}

impl AuthPrincipal {
    pub fn user_id(&self) -> i64 {
        match self {
            AuthPrincipal::User(user) | AuthPrincipal::Admin(user) => user.user_id,
        }
    }

    pub fn user(&self) -> &UserModel {
        match self {
            AuthPrincipal::User(user) | AuthPrincipal::Admin(user) => user,
        }
    }

    pub fn is_admin(&self) -> bool {
        matches!(self, AuthPrincipal::Admin(_))
    }
}

/// Enforces per-retreat tenancy for non-admin callers.
///
/// Admins bypass and get `Ok(None)`. Retreat users must hold a
/// `retreat_users` row for `retreat_id`; the row is returned so callers
/// that need role gating (team management) can inspect `role`.
/// Unknown/foreign retreat access fails closed with 403.
pub async fn ensure_retreat_membership(
    db: &DatabaseConnection,
    principal: &AuthPrincipal,
    retreat_id: i64,
) -> Result<Option<RetreatUserModel>, Response<Body>> {
    if principal.is_admin() {
        return Ok(None);
    }
    let membership: Option<RetreatUserModel> = RetreatUserEntity::find()
        .filter(RetreatUserColumn::UserId.eq(principal.user_id()))
        .filter(RetreatUserColumn::RetreatId.eq(retreat_id))
        .one(db)
        .await
        .map_err(|e| to_error_response(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    match membership {
        Some(row) => Ok(Some(row)),
        None => Err(to_error_response_with_message(
            "You do not have access to this retreat.",
            StatusCode::FORBIDDEN,
        )),
    }
}

/// Team management (invite / role change / remove) requires owner or
/// manager role. Pass the membership row from `ensure_retreat_membership`;
/// `None` (admin bypass) is allowed through.
pub fn ensure_team_manager(membership: Option<&RetreatUserModel>) -> Result<(), Response<Body>> {
    if membership.is_none() {
        return Ok(());
    }
    let is_manager: bool = membership
        .and_then(|m| m.role.as_deref())
        .map(|role| role.eq_ignore_ascii_case("owner") || role.eq_ignore_ascii_case("manager"))
        .unwrap_or(false);
    if is_manager {
        return Ok(());
    }
    Err(to_error_response_with_message(
        "Only owners or managers can manage team members.",
        StatusCode::FORBIDDEN,
    ))
}

#[derive(Clone)]
pub struct AuthUserOrAdmin(pub AuthPrincipal);

impl AuthUserOrAdmin {
    pub fn user_id(&self) -> i64 {
        self.0.user_id()
    }

    pub fn is_admin(&self) -> bool {
        self.0.is_admin()
    }
}

impl<S> FromRequestParts<S> for AuthUserOrAdmin
where
    S: Send + Sync + std::fmt::Debug + Clone + 'static,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let (user, claims) = extract_authenticated_user(parts, state).await?;

        match claims.login_type.as_str() {
            "admin" => {
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

                Ok(AuthUserOrAdmin(AuthPrincipal::Admin(user)))
            }
            "normal" => Ok(AuthUserOrAdmin(AuthPrincipal::User(user))),
            // Retreat staff manage their own profile with the same self-scope
            // enforced below (`update_user` allows self-or-admin only).
            "retreat" => Ok(AuthUserOrAdmin(AuthPrincipal::User(user))),
            _ => Err((
                StatusCode::FORBIDDEN,
                "User or admin access required".to_string(),
            )),
        }
    }
}

#[derive(Clone)]
pub struct AuthAdminOrRetreatUser(pub AuthPrincipal);

impl AuthAdminOrRetreatUser {
    pub fn user_id(&self) -> i64 {
        self.0.user_id()
    }

    pub fn user(&self) -> &UserModel {
        self.0.user()
    }

    pub fn is_admin(&self) -> bool {
        self.0.is_admin()
    }
}

impl<S> FromRequestParts<S> for AuthAdminOrRetreatUser
where
    S: Send + Sync + std::fmt::Debug + Clone + 'static,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let (user, claims) = extract_authenticated_user(parts, state).await?;

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

        match claims.login_type.as_str() {
            "admin" => {
                AdminUserEntity::find()
                    .filter(AdminUserColumn::UserId.eq(user.user_id))
                    .one(&state.database)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                    .ok_or_else(|| (StatusCode::FORBIDDEN, "Admin access revoked".to_string()))?;
                Ok(AuthAdminOrRetreatUser(AuthPrincipal::Admin(user)))
            }
            "retreat" => {
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
                Ok(AuthAdminOrRetreatUser(AuthPrincipal::User(user)))
            }
            _ => {
                return Err((
                    StatusCode::FORBIDDEN,
                    "Admin or retreat user access required".to_string(),
                ));
            }
        }

        // NOTE: per-retreat tenancy is NOT enforced here (`FromRequestParts`
        // cannot see the path `{retreat_id}`). Each CRUD handler calls
        // `ensure_retreat_membership` with its path id; admins bypass.
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
