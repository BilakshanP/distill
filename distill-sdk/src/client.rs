use crate::error::Error;
use crate::types::*;
use uuid::Uuid;

pub struct Client {
    base_url: String,
    http: reqwest::Client,
    token: Option<String>,
}

impl Client {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
            token: None,
        }
    }

    pub fn with_token(mut self, token: &str) -> Self {
        self.token = Some(token.to_string());
        self
    }

    pub fn set_token(&mut self, token: &str) {
        self.token = Some(token.to_string());
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let mut req = self
            .http
            .request(method, format!("{}{}", self.base_url, path));
        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        req
    }

    pub async fn health(&self) -> Result<HealthResponse, Error> {
        let resp = self.request(reqwest::Method::GET, "/health").send().await?;
        Self::handle(resp).await
    }

    pub async fn create_question(
        &self,
        title: &str,
        body: &str,
        tags: &[&str],
    ) -> Result<QuestionResponse, Error> {
        let resp = self
            .request(reqwest::Method::POST, "/questions")
            .json(&serde_json::json!({ "title": title, "body": body, "tags": tags }))
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn get_question(&self, id: Uuid) -> Result<QuestionResponse, Error> {
        let resp = self
            .request(reqwest::Method::GET, &format!("/questions/{}", id))
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, Error> {
        let resp = self
            .request(reqwest::Method::GET, "/questions/search")
            .query(&[("q", query)])
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn preview(&self, title: &str, body: &str) -> Result<PreviewResponse, Error> {
        let resp = self
            .request(reqwest::Method::POST, "/questions/preview")
            .json(&serde_json::json!({ "title": title, "body": body }))
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn get_answers(&self, question_id: Uuid) -> Result<Vec<AnswerResponse>, Error> {
        let resp = self
            .request(
                reqwest::Method::GET,
                &format!("/questions/{}/answers", question_id),
            )
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn edit_answer(
        &self,
        answer_id: Uuid,
        body: &str,
        message: Option<&str>,
    ) -> Result<AnswerResponse, Error> {
        let resp = self
            .request(reqwest::Method::PUT, &format!("/answers/{}", answer_id))
            .json(&serde_json::json!({ "body": body, "edit_message": message }))
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn get_history(&self, answer_id: Uuid) -> Result<Vec<EditHistoryEntry>, Error> {
        let resp = self
            .request(
                reqwest::Method::GET,
                &format!("/answers/{}/history", answer_id),
            )
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn rate_answer(
        &self,
        answer_id: Uuid,
        score: i32,
        comment: Option<&str>,
        query: Option<&str>,
    ) -> Result<RatingResponse, Error> {
        let resp = self.request(reqwest::Method::POST, &format!("/answers/{}/ratings", answer_id))
            .json(&serde_json::json!({ "score": score, "comment": comment, "rater_original_query": query }))
            .send().await?;
        Self::handle(resp).await
    }

    pub async fn get_ratings(&self, answer_id: Uuid) -> Result<Vec<RatingResponse>, Error> {
        let resp = self
            .request(
                reqwest::Method::GET,
                &format!("/answers/{}/ratings", answer_id),
            )
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn dig_deeper(
        &self,
        answer_id: Uuid,
        prompt: &str,
    ) -> Result<DigDeeperResponse, Error> {
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/answers/{}/dig-deeper", answer_id),
            )
            .json(&serde_json::json!({ "prompt": prompt }))
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn get_graph(&self) -> Result<GraphResponse, Error> {
        let resp = self.request(reqwest::Method::GET, "/graph").send().await?;
        Self::handle(resp).await
    }

    pub async fn get_node(&self, id: Uuid) -> Result<GraphResponse, Error> {
        let resp = self
            .request(reqwest::Method::GET, &format!("/graph/node/{}", id))
            .send()
            .await?;
        Self::handle(resp).await
    }

    async fn handle<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> Result<T, Error> {
        if !resp.status().is_success() {
            return Err(Error::Api {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        Ok(resp.json().await?)
    }
}
