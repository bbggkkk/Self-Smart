pub mod vllm;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }

    pub fn estimated_tokens(&self) -> u32 {
        // Rough estimate: ~4 chars per token
        (self.content.len() as u32) / 4
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Token budget tracker for managing context window
#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub max_tokens: u32,
    pub used_tokens: u32,
}

impl TokenBudget {
    pub fn new(max_tokens: u32) -> Self {
        Self {
            max_tokens,
            used_tokens: 0,
        }
    }

    pub fn remaining(&self) -> u32 {
        self.max_tokens.saturating_sub(self.used_tokens)
    }

    pub fn can_fit(&self, tokens: u32) -> bool {
        tokens <= self.remaining()
    }

    pub fn consume(&mut self, tokens: u32) {
        self.used_tokens = self.used_tokens.saturating_add(tokens);
    }

    pub fn reset(&mut self) {
        self.used_tokens = 0;
    }

    pub fn usage_percent(&self) -> f32 {
        (self.used_tokens as f32 / self.max_tokens as f32) * 100.0
    }
}

/// Conversation context manager
#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub budget: TokenBudget,
}

impl ConversationContext {
    pub fn new(max_tokens: u32) -> Self {
        Self {
            messages: Vec::new(),
            system_prompt: None,
            budget: TokenBudget::new(max_tokens),
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        let prompt = prompt.into();
        self.budget.consume(Message::system(&prompt).estimated_tokens());
        self.system_prompt = Some(prompt);
        self
    }

    pub fn add_message(&mut self, message: Message) -> bool {
        let tokens = message.estimated_tokens();
        if self.budget.can_fit(tokens) {
            self.budget.consume(tokens);
            self.messages.push(message);
            true
        } else {
            false
        }
    }

    pub fn add_user_message(&mut self, content: impl Into<String>) -> bool {
        self.add_message(Message::user(content))
    }

    pub fn add_assistant_message(&mut self, content: impl Into<String>) -> bool {
        self.add_message(Message::assistant(content))
    }

    pub fn get_messages(&self) -> Vec<Message> {
        let mut messages = Vec::new();

        if let Some(system) = &self.system_prompt {
            messages.push(Message::system(system));
        }

        messages.extend(self.messages.clone());
        messages
    }

    pub fn trim_to_budget(&mut self) {
        let mut total_tokens = 0;
        if let Some(system) = &self.system_prompt {
            total_tokens += Message::system(system).estimated_tokens();
        }

        // Keep messages from the end that fit in budget
        let mut kept_messages = Vec::new();
        for msg in self.messages.iter().rev() {
            let tokens = msg.estimated_tokens();
            if total_tokens + tokens <= self.budget.max_tokens {
                total_tokens += tokens;
                kept_messages.push(msg.clone());
            } else {
                break;
            }
        }

        kept_messages.reverse();
        self.messages = kept_messages;
        self.budget.used_tokens = total_tokens;
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.budget.reset();
        if let Some(system) = &self.system_prompt {
            self.budget.consume(Message::system(system).estimated_tokens());
        }
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, messages: Vec<Message>) -> anyhow::Result<String>;
    async fn chat_with_usage(&self, messages: Vec<Message>) -> anyhow::Result<(String, Option<Usage>)>;
    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        callback: impl FnMut(String) + Send,
    ) -> anyhow::Result<String>;
}
