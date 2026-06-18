//! LLM client — supports a `mock` provider (default) and an `openai-compatible`
//! provider that talks to any OpenAI-shaped endpoint via reqwest.

use crate::config::LlmConfig;
use crate::errors::{EvAgentError, Result};
use crate::models::{LlmMessage, LlmRequest, LlmResponse};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, messages: Vec<LlmMessage>) -> Result<LlmResponse>;
    fn provider_name(&self) -> &'static str;
}

pub fn build_client(cfg: &LlmConfig) -> Result<Arc<dyn LlmClient>> {
    match cfg.provider.as_str() {
        "mock" => Ok(Arc::new(MockLlmClient::default())),
        "openai-compatible" => {
            let api_key = std::env::var("EVAGENT_API_KEY")
                .map_err(|_| EvAgentError::Config("EVAGENT_API_KEY not set".into()))?;
            Ok(Arc::new(OpenAiCompatibleClient::new(
                cfg.base_url.clone(),
                cfg.model.clone(),
                api_key,
                cfg.max_tokens,
                cfg.temperature,
            )))
        }
        other => Err(EvAgentError::Config(format!(
            "unknown llm provider: {}",
            other
        ))),
    }
}

/// Mock LLM — returns deterministic canned responses. Lets us exercise the
/// entire dispatch pipeline without an API key.
#[derive(Default)]
pub struct MockLlmClient;

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(&self, messages: Vec<LlmMessage>) -> Result<LlmResponse> {
        let started = Instant::now();
        // Simulate latency so the TUI/Web can see "Running" before "Completed".
        tokio::time::sleep(Duration::from_millis(200)).await;

        let last = messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();
        let role_summary = messages
            .iter()
            .map(|m| m.role.clone())
            .collect::<Vec<_>>()
            .join(",");

        let reply = format!(
            "[mock-llm] processed {} message(s) ({role_summary}). Last user content ({} chars): {:60}\n\
             This is a deterministic mock response. Set llm.provider=\"openai-compatible\" in config.yaml and EVAGENT_API_KEY in .env to use a real model.",
            messages.len(),
            last.len(),
            last.chars().take(60).collect::<String>()
        );

        let tokens = (reply.len() as u64 + last.len() as u64) / 4;
        let _ = started;
        Ok(LlmResponse {
            choices: vec![crate::models::LlmChoice {
                message: LlmMessage {
                    role: "assistant".into(),
                    content: reply,
                },
            }],
            usage: crate::models::LlmUsage {
                total_tokens: tokens.max(10),
            },
        })
    }

    fn provider_name(&self) -> &'static str {
        "mock"
    }
}

/// OpenAI-compatible HTTP client (works with deepseek, openrouter, openai, etc.).
pub struct OpenAiCompatibleClient {
    base_url: String,
    model: String,
    api_key: String,
    max_tokens: u32,
    temperature: f32,
    http: reqwest::Client,
}

impl OpenAiCompatibleClient {
    pub fn new(
        base_url: String,
        model: String,
        api_key: String,
        max_tokens: u32,
        temperature: f32,
    ) -> Self {
        Self {
            base_url,
            model,
            api_key,
            max_tokens,
            temperature,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("reqwest client"),
        }
    }
}

#[async_trait]
impl LlmClient for OpenAiCompatibleClient {
    async fn complete(&self, messages: Vec<LlmMessage>) -> Result<LlmResponse> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let req = LlmRequest {
            model: self.model.clone(),
            messages,
            max_tokens: Some(self.max_tokens),
            temperature: Some(self.temperature),
        };
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .map_err(|e| EvAgentError::Llm(format!("POST {}: {}", url, e)))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(EvAgentError::Llm(format!(
                "HTTP {} {}: {}",
                status,
                url,
                body.chars().take(500).collect::<String>()
            )));
        }
        let parsed: LlmResponse = resp
            .json()
            .await
            .map_err(|e| EvAgentError::Llm(format!("decode body: {}", e)))?;
        Ok(parsed)
    }

    fn provider_name(&self) -> &'static str {
        "openai-compatible"
    }
}
