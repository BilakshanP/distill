use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Serialize, utoipa::ToSchema)]
pub struct TagCount {
    pub tag: String,
    pub count: i64,
}

#[derive(Deserialize)]
pub struct TagParams {
    pub q: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[utoipa::path(get, path = "/tags", responses((status = 200, body = Vec<TagCount>)), tag = "tags", security(()))]
pub async fn list_tags(
    State(state): State<AppState>,
    Query(params): Query<TagParams>,
) -> Result<Json<Vec<TagCount>>, StatusCode> {
    let limit = params.limit.min(100);

    let rows = if let Some(ref q) = params.q {
        let pattern = format!("%{}%", q.to_lowercase());
        sqlx::query_as::<_, (String, i64)>(
            r#"SELECT tag, COUNT(*) as count
               FROM (SELECT unnest(tags) AS tag FROM questions) t
               WHERE LOWER(tag) LIKE $1
               GROUP BY tag ORDER BY count DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(&pattern)
        .bind(limit)
        .bind(params.offset)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, (String, i64)>(
            r#"SELECT tag, COUNT(*) as count
               FROM (SELECT unnest(tags) AS tag FROM questions) t
               GROUP BY tag ORDER BY count DESC
               LIMIT $1 OFFSET $2"#,
        )
        .bind(limit)
        .bind(params.offset)
        .fetch_all(&state.db)
        .await
    }
    .map_err(|e| {
        tracing::error!("list tags failed: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(
        rows.into_iter()
            .map(|r| TagCount {
                tag: r.0,
                count: r.1,
            })
            .collect(),
    ))
}
