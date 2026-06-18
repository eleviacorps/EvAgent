//! Intent router — classifies a user prompt into one of the registered domains.
//!
//! Three-stage pipeline per spec:
//! 1. Regex pattern matching (confidence > 0.8 wins immediately)
//! 2. Token-frequency cosine similarity (confidence > 0.6 wins)
//! 3. LLM-based classification (final fallback)
//!
//! Domain definitions come from the YAML files under `domains/<name>/domain.yaml`.

use crate::errors::{EvAgentError, Result};
use crate::llm_client::LlmClient;
use crate::models::{Domain, LlmMessage};
use parking_lot::RwLock;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone)]
pub struct IntentRouter {
    domains: Arc<RwLock<Vec<Domain>>>,
    min_confidence: f32,
    llm: Option<Arc<dyn LlmClient>>,
}

#[derive(Debug, Clone)]
pub struct RouteResult {
    pub domain: String,
    pub confidence: f32,
    pub method: RouteMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteMethod {
    Regex,
    Embedding,
    Llm,
    Default,
}

impl IntentRouter {
    pub fn new(min_confidence: f32) -> Self {
        Self {
            domains: Arc::new(RwLock::new(Vec::new())),
            min_confidence,
            llm: None,
        }
    }

    pub fn set_llm(&mut self, llm: Arc<dyn LlmClient>) {
        self.llm = Some(llm);
    }

    pub fn register(&self, domain: Domain) {
        self.domains.write().push(domain);
    }

    pub fn domains(&self) -> Vec<Domain> {
        self.domains.read().clone()
    }

    /// Classify a prompt. Always returns a RouteResult — falls back to "general"
    /// if nothing matches above threshold.
    pub async fn route(&self, prompt: &str) -> Result<RouteResult> {
        let domains = self.domains.read().clone();
        if domains.is_empty() {
            return Ok(RouteResult {
                domain: "general".into(),
                confidence: 0.0,
                method: RouteMethod::Default,
            });
        }

        // Stage 1: regex
        if let Some(r) = self.route_regex(prompt, &domains) {
            return Ok(r);
        }

        // Stage 2: embedding (token-frequency cosine similarity)
        if let Some(r) = self.route_embedding(prompt, &domains) {
            return Ok(r);
        }

        // Stage 3: LLM
        if let Some(llm) = &self.llm {
            if let Ok(r) = self.route_llm(prompt, &domains, llm).await {
                if r.confidence >= self.min_confidence {
                    return Ok(r);
                }
            }
        }

        // Pick highest-priority domain as a default-of-last-resort.
        let fallback = domains
            .iter()
            .max_by_key(|d| d.priority)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| "general".into());
        Ok(RouteResult {
            domain: fallback,
            confidence: 0.0,
            method: RouteMethod::Default,
        })
    }

    fn route_regex(&self, prompt: &str, domains: &[Domain]) -> Option<RouteResult> {
        let lower = prompt.to_lowercase();
        for d in domains {
            for pat in &d.patterns {
                let re = match Regex::new(pat) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                if re.is_match(&lower) {
                    return Some(RouteResult {
                        domain: d.name.clone(),
                        confidence: 0.9,
                        method: RouteMethod::Regex,
                    });
                }
            }
        }
        None
    }

    fn route_embedding(&self, prompt: &str, domains: &[Domain]) -> Option<RouteResult> {
        let prompt_vec = term_freq_vec(prompt);
        let mut best: Option<(String, f32)> = None;
        for d in domains {
            // Combine domain name + pattern strings + agent names into a "document"
            let doc = format!(
                "{} {} {} {}",
                d.name,
                d.patterns.join(" "),
                d.agents.join(" "),
                d.skills.join(" ")
            );
            let dv = term_freq_vec(&doc);
            let sim = cosine(&prompt_vec, &dv);
            match &best {
                Some((_, c)) if *c >= sim => {}
                _ => best = Some((d.name.clone(), sim)),
            }
        }
        if let Some((name, sim)) = best {
            if sim >= 0.6 {
                return Some(RouteResult {
                    domain: name,
                    confidence: sim,
                    method: RouteMethod::Embedding,
                });
            }
        }
        None
    }

    async fn route_llm(
        &self,
        prompt: &str,
        domains: &[Domain],
        llm: &Arc<dyn LlmClient>,
    ) -> Result<RouteResult> {
        let names: Vec<&str> = domains.iter().map(|d| d.name.as_str()).collect();
        let system = "You are an intent router. Reply with exactly one domain name from the list. No explanation.".to_string();
        let user = format!(
            "Domains: {}\nPrompt: {}\nReply with exactly one domain name.",
            names.join(", "),
            prompt
        );
        let msgs = vec![
            LlmMessage {
                role: "system".into(),
                content: system,
            },
            LlmMessage {
                role: "user".into(),
                content: user,
            },
        ];
        let resp = llm.complete(msgs).await?;
        let content = resp
            .choices
            .first()
            .map(|c| c.message.content.trim().to_lowercase())
            .unwrap_or_default();
        let matched = domains.iter().find(|d| content.contains(&d.name));
        match matched {
            Some(d) => Ok(RouteResult {
                domain: d.name.clone(),
                confidence: 0.7,
                method: RouteMethod::Llm,
            }),
            None => Err(EvAgentError::RoutingNoMatch),
        }
    }
}

/// Build a term-frequency vector (HashMap<token, count>) for cosine similarity.
fn term_freq_vec(text: &str) -> HashMap<String, f32> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    for tok in tokenize(text) {
        *counts.entry(tok).or_insert(0) += 1;
    }
    let total = counts.values().sum::<u32>() as f32;
    if total == 0.0 {
        return HashMap::new();
    }
    counts
        .into_iter()
        .map(|(k, v)| (k, v as f32 / total))
        .collect()
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 2)
        .map(|s| s.to_string())
        .collect()
}

fn cosine(a: &HashMap<String, f32>, b: &HashMap<String, f32>) -> f32 {
    let keys: HashSet<&String> = a.keys().chain(b.keys()).collect();
    let dot: f32 = keys
        .iter()
        .filter_map(|k| a.get(*k).zip(b.get(*k)).map(|(x, y)| x * y))
        .sum();
    let norm_a: f32 = a.values().map(|v| v * v).sum::<f32>().sqrt();
    let norm_b: f32 = b.values().map(|v| v * v).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
