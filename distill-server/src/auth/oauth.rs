use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Redirect,
    Json,
};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    EndpointNotSet, EndpointSet, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;

use crate::AppState;

use super::jwt;

#[derive(Deserialize)]
struct GitHubUser {
    id: i64,
    login: String,
    email: Option<String>,
    avatar_url: Option<String>,
}

type GhOAuthClient = oauth2::Client<
    oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>,
    oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>,
    oauth2::StandardTokenIntrospectionResponse<
        oauth2::EmptyExtraTokenFields,
        oauth2::basic::BasicTokenType,
    >,
    oauth2::StandardRevocableToken,
    oauth2::StandardErrorResponse<oauth2::RevocationErrorResponseType>,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

fn oauth_client(state: &AppState) -> GhOAuthClient {
    BasicClient::new(ClientId::new(state.github_client_id.clone()))
        .set_client_secret(ClientSecret::new(state.github_client_secret.clone()))
        .set_auth_uri(AuthUrl::new("https://github.com/login/oauth/authorize".into()).unwrap())
        .set_token_uri(TokenUrl::new("https://github.com/login/oauth/access_token".into()).unwrap())
        .set_redirect_uri(
            RedirectUrl::new(format!("{}/auth/github/callback", state.base_url)).unwrap(),
        )
}

pub async fn github_login(State(state): State<AppState>) -> Redirect {
    let client = oauth_client(&state);
    let (auth_url, _csrf) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:user".into()))
        .add_scope(Scope::new("user:email".into()))
        .url();
    Redirect::to(auth_url.as_str())
}

#[derive(Deserialize)]
pub struct CallbackParams {
    code: String,
}

pub async fn github_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = oauth_client(&state);

    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let token = client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(&http_client)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let access_token = token.access_token().secret();

    // Fetch GitHub user profile
    let gh_user: GitHubUser = reqwest::Client::new()
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "distill")
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Upsert user
    let user = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid)>(
        r#"INSERT INTO users (provider, provider_id, display_name, email, avatar_url)
           VALUES ('github', $1, $2, $3, $4)
           ON CONFLICT (provider, provider_id) DO UPDATE SET
             display_name = EXCLUDED.display_name,
             email = EXCLUDED.email,
             avatar_url = EXCLUDED.avatar_url
           RETURNING id, tenant_id"#,
    )
    .bind(gh_user.id.to_string())
    .bind(&gh_user.login)
    .bind(&gh_user.email)
    .bind(&gh_user.avatar_url)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tenant_id = if user.1 == uuid::Uuid::nil() {
        None
    } else {
        Some(user.1)
    };

    let jwt = jwt::create_token_with_tenant(user.0, tenant_id, &state.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({ "token": jwt })))
}

// === Google OAuth ===

#[derive(Deserialize)]
struct GoogleUser {
    sub: String,
    name: Option<String>,
    email: Option<String>,
    picture: Option<String>,
}

fn google_oauth_client(state: &AppState) -> Option<GhOAuthClient> {
    let client_id = state.google_client_id.as_ref()?;
    let client_secret = state.google_client_secret.as_ref()?;
    Some(
        BasicClient::new(ClientId::new(client_id.clone()))
            .set_client_secret(ClientSecret::new(client_secret.clone()))
            .set_auth_uri(
                AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".into()).unwrap(),
            )
            .set_token_uri(TokenUrl::new("https://oauth2.googleapis.com/token".into()).unwrap())
            .set_redirect_uri(
                RedirectUrl::new(format!("{}/auth/google/callback", state.base_url)).unwrap(),
            ),
    )
}

pub async fn google_login(State(state): State<AppState>) -> Result<Redirect, StatusCode> {
    let client = google_oauth_client(&state).ok_or(StatusCode::NOT_FOUND)?;
    let (auth_url, _csrf) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".into()))
        .add_scope(Scope::new("email".into()))
        .add_scope(Scope::new("profile".into()))
        .url();
    Ok(Redirect::to(auth_url.as_str()))
}

pub async fn google_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = google_oauth_client(&state).ok_or(StatusCode::NOT_FOUND)?;

    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let token = client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(&http_client)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let access_token = token.access_token().secret();

    let google_user: GoogleUser = reqwest::Client::new()
        .get("https://openidconnect.googleapis.com/v1/userinfo")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let display_name = google_user
        .name
        .unwrap_or_else(|| google_user.email.clone().unwrap_or("Google User".into()));

    let user = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid)>(
        r#"INSERT INTO users (provider, provider_id, display_name, email, avatar_url)
           VALUES ('google', $1, $2, $3, $4)
           ON CONFLICT (provider, provider_id) DO UPDATE SET
             display_name = EXCLUDED.display_name,
             email = EXCLUDED.email,
             avatar_url = EXCLUDED.avatar_url
           RETURNING id, tenant_id"#,
    )
    .bind(&google_user.sub)
    .bind(&display_name)
    .bind(&google_user.email)
    .bind(&google_user.picture)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tenant_id = if user.1 == uuid::Uuid::nil() {
        None
    } else {
        Some(user.1)
    };

    let jwt = jwt::create_token_with_tenant(user.0, tenant_id, &state.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({ "token": jwt })))
}
