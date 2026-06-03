use std::env;

pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub auto_migrate: bool,
    pub jwt_secret: String,
    pub github_client_id: String,
    pub github_client_secret: String,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub base_url: String,
    pub llm_chat_model: Option<String>,
    pub llm_embedding_model: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .expect("PORT must be a number"),
            auto_migrate: env::var("AUTO_MIGRATE")
                .unwrap_or_else(|_| "false".into())
                .parse()
                .unwrap_or(false),
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            github_client_id: env::var("GITHUB_CLIENT_ID").expect("GITHUB_CLIENT_ID must be set"),
            github_client_secret: env::var("GITHUB_CLIENT_SECRET")
                .expect("GITHUB_CLIENT_SECRET must be set"),
            google_client_id: env::var("GOOGLE_CLIENT_ID").ok().filter(|s| !s.is_empty()),
            google_client_secret: env::var("GOOGLE_CLIENT_SECRET")
                .ok()
                .filter(|s| !s.is_empty()),
            base_url: env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into()),
            llm_chat_model: env::var("LLM_CHAT_MODEL").ok().filter(|s| !s.is_empty()),
            llm_embedding_model: env::var("LLM_EMBEDDING_MODEL")
                .ok()
                .filter(|s| !s.is_empty()),
        }
    }
}
