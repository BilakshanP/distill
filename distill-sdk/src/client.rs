use crate::error::Error;
use crate::types::*;
use std::collections::HashMap;
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

    async fn handle<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> Result<T, Error> {
        if !resp.status().is_success() {
            return Err(Error::Api {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        Ok(resp.json().await?)
    }

    async fn handle_no_body(resp: reqwest::Response) -> Result<(), Error> {
        if !resp.status().is_success() {
            return Err(Error::Api {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        Ok(())
    }

    // Health
    pub async fn health(&self) -> Result<HealthResponse, Error> {
        let resp = self.request(reqwest::Method::GET, "/health").send().await?;
        Self::handle(resp).await
    }

    // Auth
    pub async fn delete_account(&self) -> Result<(), Error> {
        let resp = self.request(reqwest::Method::DELETE, "/me").send().await?;
        Self::handle_no_body(resp).await
    }

    // Questions
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

    pub async fn search(
        &self,
        query: &str,
        tags: Option<&str>,
    ) -> Result<Vec<SearchResult>, Error> {
        let mut req = self
            .request(reqwest::Method::GET, "/questions/search")
            .query(&[("q", query)]);
        if let Some(t) = tags {
            req = req.query(&[("tags", t)]);
        }
        let resp = req.send().await?;
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

    pub async fn link_questions(
        &self,
        question_id: Uuid,
        target_id: Uuid,
        link_type: &str,
    ) -> Result<LinkResponse, Error> {
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/questions/{}/link", question_id),
            )
            .json(&serde_json::json!({ "target_question_id": target_id, "link_type": link_type }))
            .send()
            .await?;
        Self::handle(resp).await
    }

    // Answers
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

    pub async fn mark_stale(
        &self,
        answer_id: Uuid,
        reason: Option<&str>,
    ) -> Result<AnswerResponse, Error> {
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/answers/{}/mark-stale", answer_id),
            )
            .json(&serde_json::json!({ "reason": reason }))
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn dig_deeper(
        &self,
        answer_id: Uuid,
        prompt: &str,
        include_comments: bool,
    ) -> Result<DigDeeperResponse, Error> {
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/answers/{}/dig-deeper", answer_id),
            )
            .json(&serde_json::json!({ "prompt": prompt, "include_comments": include_comments }))
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn get_deep_dives(&self, answer_id: Uuid) -> Result<Vec<DigDeeperResponse>, Error> {
        let resp = self
            .request(
                reqwest::Method::GET,
                &format!("/answers/{}/deep-dives", answer_id),
            )
            .send()
            .await?;
        Self::handle(resp).await
    }

    // Ratings
    pub async fn rate_answer(
        &self,
        answer_id: Uuid,
        score: i32,
        comment: Option<&str>,
        query: Option<&str>,
    ) -> Result<RatingResponse, Error> {
        let resp = self
            .request(reqwest::Method::POST, &format!("/answers/{}/ratings", answer_id))
            .json(&serde_json::json!({ "score": score, "comment": comment, "rater_original_query": query }))
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn get_ratings(
        &self,
        answer_id: Uuid,
        after: Option<&str>,
    ) -> Result<Paginated<RatingResponse>, Error> {
        let mut req = self.request(
            reqwest::Method::GET,
            &format!("/answers/{}/ratings", answer_id),
        );
        if let Some(cursor) = after {
            req = req.query(&[("after", cursor)]);
        }
        let resp = req.send().await?;
        Self::handle(resp).await
    }

    pub async fn redact_rating(&self, answer_id: Uuid) -> Result<(), Error> {
        let resp = self
            .request(
                reqwest::Method::PUT,
                &format!("/answers/{}/ratings/redact", answer_id),
            )
            .send()
            .await?;
        Self::handle_no_body(resp).await
    }

    // Contradictions
    pub async fn flag_contradiction(
        &self,
        answer_id: Uuid,
        contradicts_id: Uuid,
        explanation: &str,
    ) -> Result<ContradictionResponse, Error> {
        let resp = self
            .request(reqwest::Method::POST, &format!("/answers/{}/flag-contradiction", answer_id))
            .json(&serde_json::json!({ "contradicts_answer_id": contradicts_id, "explanation": explanation }))
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn get_contradictions(
        &self,
        answer_id: Uuid,
    ) -> Result<Vec<ContradictionResponse>, Error> {
        let resp = self
            .request(
                reqwest::Method::GET,
                &format!("/answers/{}/contradictions", answer_id),
            )
            .send()
            .await?;
        Self::handle(resp).await
    }

    // Comments
    pub async fn create_question_comment(
        &self,
        question_id: Uuid,
        body: &str,
    ) -> Result<CommentResponse, Error> {
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/questions/{}/comments", question_id),
            )
            .json(&serde_json::json!({ "body": body }))
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn get_question_comments(
        &self,
        question_id: Uuid,
    ) -> Result<Vec<CommentResponse>, Error> {
        let resp = self
            .request(
                reqwest::Method::GET,
                &format!("/questions/{}/comments", question_id),
            )
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn create_answer_comment(
        &self,
        answer_id: Uuid,
        body: &str,
    ) -> Result<CommentResponse, Error> {
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/answers/{}/comments", answer_id),
            )
            .json(&serde_json::json!({ "body": body }))
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn get_answer_comments(
        &self,
        answer_id: Uuid,
    ) -> Result<Vec<CommentResponse>, Error> {
        let resp = self
            .request(
                reqwest::Method::GET,
                &format!("/answers/{}/comments", answer_id),
            )
            .send()
            .await?;
        Self::handle(resp).await
    }

    // Tags
    pub async fn list_tags(
        &self,
        query: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<TagCount>, Error> {
        let mut req = self.request(reqwest::Method::GET, "/tags");
        if let Some(q) = query {
            req = req.query(&[("q", q)]);
        }
        if let Some(l) = limit {
            req = req.query(&[("limit", l.to_string())]);
        }
        let resp = req.send().await?;
        Self::handle(resp).await
    }

    // Graph
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

    // Admin
    pub async fn get_config(&self) -> Result<ConfigResponse, Error> {
        let resp = self
            .request(reqwest::Method::GET, "/admin/config")
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn update_config(
        &self,
        config: HashMap<String, String>,
    ) -> Result<ConfigResponse, Error> {
        let resp = self
            .request(reqwest::Method::PUT, "/admin/config")
            .json(&serde_json::json!({ "config": config }))
            .send()
            .await?;
        Self::handle(resp).await
    }

    pub async fn get_admin_contradictions(
        &self,
        after: Option<&str>,
    ) -> Result<Paginated<ContradictionResponse>, Error> {
        let mut req = self.request(reqwest::Method::GET, "/admin/contradictions");
        if let Some(cursor) = after {
            req = req.query(&[("after", cursor)]);
        }
        let resp = req.send().await?;
        Self::handle(resp).await
    }
}
