use super::{ChatRequest, ChatResponse, LlmClient, Message, Usage};
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;

pub struct VllmClient {
    client: Client,
    endpoint: String,
    model: String,
}

impl VllmClient {
    pub fn new(endpoint: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.endpoint);
        match self.client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/v1/models", self.endpoint);
        let resp: Value = self.client.get(&url).send().await?.json().await?;
        let models = resp["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| m["id"].as_str().map(String::from))
            .collect();
        Ok(models)
    }

    pub async fn chat_with_params(
        &self,
        messages: Vec<Message>,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<ChatResponse> {
        let url = format!("{}/v1/chat/completions", self.endpoint);
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(temperature),
            max_tokens: Some(max_tokens),
            stream: Some(false),
        };

        let response: ChatResponse = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        Ok(response)
    }
}

#[async_trait::async_trait]
impl LlmClient for VllmClient {
    async fn chat(&self, messages: Vec<Message>) -> Result<String> {
        let response = self.chat_with_params(messages, 0.7, 4096).await?;
        Ok(response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    async fn chat_with_usage(&self, messages: Vec<Message>) -> Result<(String, Option<Usage>)> {
        let response = self.chat_with_params(messages, 0.7, 4096).await?;
        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        Ok((content, response.usage))
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        mut callback: impl FnMut(String) + Send,
    ) -> Result<String> {
        let url = format!("{}/v1/chat/completions", self.endpoint);
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: Some(true),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        let text = response.text().await?;
        let mut full_content = String::new();

        for line in text.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    break;
                }
                if let Ok(json) = serde_json::from_str::<Value>(data) {
                    if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                        full_content.push_str(content);
                        callback(content.to_string());
                    }
                }
            }
        }

        Ok(full_content)
    }
}
