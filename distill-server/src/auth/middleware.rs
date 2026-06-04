use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};
use uuid::Uuid;

use crate::AppState;

use super::jwt;

pub struct AuthUser {
    pub user_id: Uuid,
}

impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let claims = jwt::validate_token(token, &app_state.jwt_secret)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        // Verify user still exists
        let exists: Option<(bool,)> = sqlx::query_as("SELECT true FROM users WHERE id = $1")
            .bind(claims.sub)
            .fetch_optional(&app_state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if exists.is_none() {
            return Err(StatusCode::UNAUTHORIZED);
        }

        // Set tenant context for RLS if present in JWT
        if let Some(tid) = claims.tenant_id {
            crate::routes::set_tenant(&app_state.db, tid).await;
        }

        Ok(AuthUser {
            user_id: claims.sub,
        })
    }
}

use axum::extract::FromRef;

pub struct AdminUser {
    pub user_id: Uuid,
}

impl<S> FromRequestParts<S> for AdminUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let claims = jwt::validate_token(token, &app_state.jwt_secret)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        // Check role
        let role: Option<(String,)> = sqlx::query_as("SELECT role FROM users WHERE id = $1")
            .bind(claims.sub)
            .fetch_optional(&app_state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        match role {
            Some((r,)) if r == "admin" => Ok(AdminUser {
                user_id: claims.sub,
            }),
            _ => Err(StatusCode::FORBIDDEN),
        }
    }
}
