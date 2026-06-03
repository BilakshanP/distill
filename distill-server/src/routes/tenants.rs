use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::middleware::AdminUser, AppState};

#[derive(Serialize, utoipa::ToSchema)]
pub struct TenantResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
}

#[utoipa::path(post, path = "/admin/tenants", request_body = CreateTenantRequest, responses((status = 201, body = TenantResponse)), tag = "tenants")]
pub async fn create_tenant(
    State(state): State<AppState>,
    _auth: AdminUser,
    Json(req): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<TenantResponse>), StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, String, String, chrono::DateTime<chrono::Utc>)>(
        "INSERT INTO tenants (name, slug) VALUES ($1, $2) RETURNING id, name, slug, created_at",
    )
    .bind(&req.name)
    .bind(&req.slug)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::CONFLICT)?;

    Ok((
        StatusCode::CREATED,
        Json(TenantResponse {
            id: row.0,
            name: row.1,
            slug: row.2,
            created_at: row.3,
        }),
    ))
}

#[utoipa::path(get, path = "/admin/tenants", responses((status = 200, body = Vec<TenantResponse>)), tag = "tenants")]
pub async fn list_tenants(
    State(state): State<AppState>,
    _auth: AdminUser,
) -> Result<Json<Vec<TenantResponse>>, StatusCode> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, name, slug, created_at FROM tenants ORDER BY created_at",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(
        rows.into_iter()
            .map(|r| TenantResponse {
                id: r.0,
                name: r.1,
                slug: r.2,
                created_at: r.3,
            })
            .collect(),
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AssignTenantRequest {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
}

#[utoipa::path(put, path = "/admin/tenants/assign", request_body = AssignTenantRequest, responses((status = 204)), tag = "tenants")]
pub async fn assign_tenant(
    State(state): State<AppState>,
    _auth: AdminUser,
    Json(req): Json<AssignTenantRequest>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("UPDATE users SET tenant_id = $1 WHERE id = $2")
        .bind(req.tenant_id)
        .bind(req.user_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
