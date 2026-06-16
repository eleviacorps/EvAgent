use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use regex::RegexSet;
use tracing::{debug, info, warn};

use crate::errors::{HermesError, HermesResult};
use crate::models::{RegisteredDomain, RouterOutput};

/// Minimum confidence threshold for routing decisions.
const MIN_CONFIDENCE_DEFAULT: f64 = 0.6;

/// Simple TF-IDF-like vector for computing similarity between text and patterns.
#[derive(Clone)]
struct IntentVector {
    domain: String,
    patterns: Vec<String>,
    /// Precomputed term frequencies for pattern words
    term_frequencies: HashMap<String, f64>,
    /// Total unique terms in this domain's patterns
    total_terms: usize,
}

impl IntentVector {
    fn new(domain: &str, patterns: &[String]) -> Self {
        let mut term_frequencies: HashMap<String, f64> = HashMap::new();
        for pattern in patterns {
            for word in tokenize(pattern) {
                *term_frequencies.entry(word).or_insert(0.0) += 1.0;
            }
        }
        let total_terms = term_frequencies.len();

        // Normalize by total patterns (crude IDF approximation)
        let num_patterns = patterns.len().max(1) as f64;
        for val in term_frequencies.values_mut() {
            *val /= num_patterns;
        }

        Self {
            domain: domain.to_string(),
            patterns: patterns.to_vec(),
            term_frequencies,
            total_terms,
        }
    }

    /// Compute cosine similarity between this domain's patterns and a query.
    fn cosine_similarity(&self, query_terms: &HashMap<String, f64>) -> f64 {
        let mut dot_product = 0.0;
        let mut mag_a = 0.0;
        let mut mag_b = 0.0;

        for (term, freq) in &self.term_frequencies {
            mag_a += freq * freq;
            if let Some(qfreq) = query_terms.get(term) {
                dot_product += freq * qfreq;
            }
        }

        for (_, freq) in query_terms {
            mag_b += freq * freq;
        }

        mag_a = mag_a.sqrt();
        mag_b = mag_b.sqrt();

        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }

        dot_product / (mag_a * mag_b)
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty() && s.len() > 1)
        .map(|s| s.to_string())
        .collect()
}

fn compute_query_terms(query: &str) -> HashMap<String, f64> {
    let tokens = tokenize(query);
    let total = tokens.len() as f64;
    let mut freqs: HashMap<String, f64> = HashMap::new();
    for token in tokens {
        *freqs.entry(token).or_insert(0.0) += 1.0;
    }
    for val in freqs.values_mut() {
        *val /= total;
    }
    freqs
}

/// The Phoenix of intent routing: pure speed regex first, then similarity, then optional LLM fallback.
pub struct IntentRouter {
    /// Registered domains with their patterns and agents
    domains: Arc<RwLock<Vec<RegisteredDomain>>>,
    /// Precompiled RegexSet for all patterns across all domains
    regex_set: Arc<RwLock<Option<RegexSet>>>,
    /// Domain name -> list of pattern strings (for mapping matches back)
    domain_patterns: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Precomputed intent vectors for similarity fallback
    intent_vectors: Arc<RwLock<Vec<IntentVector>>>,
    /// Minimum confidence threshold
    min_confidence: f64,
}

impl IntentRouter {
    pub fn new(min_confidence: f64) -> Self {
        Self {
            domains: Arc::new(RwLock::new(Vec::new())),
            regex_set: Arc::new(RwLock::new(None)),
            domain_patterns: Arc::new(RwLock::new(HashMap::new())),
            intent_vectors: Arc::new(RwLock::new(Vec::new())),
            min_confidence,
        }
    }

    /// Register a domain with its patterns and available agents.
    pub fn register_domain(&self, domain: RegisteredDomain) -> HermesResult<()> {
        let mut domains = self.domains.write();
        // Check for duplicate
        if domains.iter().any(|d| d.name == domain.name) {
            return Err(HermesError::router(format!(
                "Domain '{}' is already registered",
                domain.name
            )));
        }

        let patterns = domain.patterns.clone();
        let name = domain.name.clone();

        domains.push(domain);

        // Update domain_patterns map
        let mut dp = self.domain_patterns.write();
        dp.insert(name.clone(), patterns.clone());

        // Rebuild regex set
        self.rebuild_regex_set()?;

        // Rebuild intent vectors
        self.rebuild_intent_vectors();

        debug!(
            "Registered domain '{}' with {} patterns",
            name,
            patterns.len()
        );
        Ok(())
    }

    /// Unregister a domain.
    pub fn unregister_domain(&self, name: &str) -> HermesResult<()> {
        let mut domains = self.domains.write();
        let before = domains.len();
        domains.retain(|d| d.name != name);
        if domains.len() == before {
            return Err(HermesError::router(format!("Domain '{}' not found", name)));
        }

        let mut dp = self.domain_patterns.write();
        dp.remove(name);

        self.rebuild_regex_set()?;
        self.rebuild_intent_vectors();
        info!("Unregistered domain '{}'", name);
        Ok(())
    }

    fn rebuild_regex_set(&self) -> HermesResult<()> {
        let domains = self.domains.read();
        let all_patterns: Vec<String> = domains
            .iter()
            .flat_map(|d| d.patterns.iter().cloned())
            .collect();

        let set = if all_patterns.is_empty() {
            None
        } else {
            Some(
                RegexSet::new(&all_patterns)
                    .map_err(|e| HermesError::router_with("Failed to build regex set", e))?,
            )
        };

        let mut rs = self.regex_set.write();
        *rs = set;

        // Also rebuild the pattern-index mapping
        let mut dp = self.domain_patterns.write();
        dp.clear();
        for domain in domains.iter() {
            dp.insert(domain.name.clone(), domain.patterns.clone());
        }

        Ok(())
    }

    fn rebuild_intent_vectors(&self) {
        let domains = self.domains.read();
        let mut vectors = Vec::new();
        for domain in domains.iter() {
            if !domain.patterns.is_empty() {
                vectors.push(IntentVector::new(&domain.name, &domain.patterns));
            }
        }
        let mut iv = self.intent_vectors.write();
        *iv = vectors;
    }

    /// Route a user prompt to the best-matching domain.
    /// Uses regex matching first (O(n) via RegexSet), then cosine similarity fallback.
    pub fn route(&self, prompt: &str) -> HermesResult<RouterOutput> {
        let prompt_lower = prompt.to_lowercase();

        // Phase 1: Regex matching (blazing fast, O(n) for all patterns)
        if let Some(ref regex_set) = *self.regex_set.read() {
            let matches: Vec<_> = regex_set.matches(&prompt_lower).into_iter().collect();

            if !matches.is_empty() {
                // Find which domain(s) matched
                let dp = self.domain_patterns.read();
                let mut domain_scores: HashMap<String, (usize, Vec<String>)> = HashMap::new();

                // Build flat pattern list with domain mapping
                let flat_patterns: Vec<(String, String)> = {
                    let domains = self.domains.read();
                    domains
                        .iter()
                        .flat_map(|d| {
                            d.patterns
                                .iter()
                                .map(move |p| (d.name.clone(), p.clone()))
                        })
                        .collect()
                };

                for &match_idx in &matches {
                    if match_idx < flat_patterns.len() {
                        let (domain_name, pattern) = &flat_patterns[match_idx];
                        let entry = domain_scores
                            .entry(domain_name.clone())
                            .or_insert((0, Vec::new()));
                        entry.0 += 1;
                        entry.1.push(pattern.clone());
                    }
                }

                // Find best domain by match count
                if let Some((best_domain, (count, matched_patterns))) = domain_scores
                    .iter()
                    .max_by_key(|(_, (c, _))| *c)
                {
                    // Calculate confidence based on match ratio
                    let total_domain_patterns = dp
                        .get(best_domain)
                        .map(|p| p.len())
                        .unwrap_or(1)
                        .max(1);
                    let confidence = (*count as f64 / total_domain_patterns as f64).min(1.0);

                    let domains = self.domains.read();
                    let agent_candidates = domains
                        .iter()
                        .find(|d| d.name == *best_domain)
                        .map(|d| d.agents.clone())
                        .unwrap_or_default();

                    if confidence >= self.min_confidence {
                        debug!(
                            "Regex route: '{}' -> '{}' (confidence: {:.2}, patterns matched: {})",
                            prompt_lower, best_domain, confidence, count
                        );

                        return Ok(RouterOutput {
                            domain: best_domain.clone(),
                            agent_candidates,
                            confidence,
                            matched_pattern: matched_patterns.first().cloned(),
                            llm_fallback_used: false,
                        });
                    }

                    // Below threshold — continue to similarity fallback
                    debug!(
                        "Regex match below threshold ({:.2} < {:.2}), trying similarity",
                        confidence, self.min_confidence
                    );
                }
            }
        }

        // Phase 2: Cosine similarity fallback
        let query_terms = compute_query_terms(prompt);
        let vectors = self.intent_vectors.read();

        if !vectors.is_empty() && !query_terms.is_empty() {
            let mut scored: Vec<(String, f64, Vec<String>)> = vectors
                .iter()
                .map(|v| {
                    let sim = v.cosine_similarity(&query_terms);
                    (v.domain.clone(), sim, v.patterns.clone())
                })
                .collect();

            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            if let Some((best_domain, confidence, matched_patterns)) = scored.first() {
                if *confidence >= self.min_confidence {
                    let domains = self.domains.read();
                    let agent_candidates = domains
                        .iter()
                        .find(|d| d.name == *best_domain)
                        .map(|d| d.agents.clone())
                        .unwrap_or_default();

                    debug!(
                        "Similarity route: '{}' -> '{}' (confidence: {:.2})",
                        prompt_lower, best_domain, confidence
                    );

                    return Ok(RouterOutput {
                        domain: best_domain.clone(),
                        agent_candidates,
                        confidence: *confidence,
                        matched_pattern: matched_patterns.first().cloned(),
                        llm_fallback_used: false,
                    });
                }
            }
        }

        // Phase 3: LLM fallback would go here
        // For now, return LowConfidence error with candidates
        let domains = self.domains.read();
        let candidates: Vec<String> = domains.iter().map(|d| d.name.clone()).collect();

        warn!(
            "Low confidence: no domain matched '{}' above threshold {}. Available domains: {:?}",
            prompt, self.min_confidence, candidates
        );

        Err(HermesError::router(format!(
            "LowConfidence: unable to route '{}' with confidence >= {}. Available domains: {:?}. Consider expanding domain patterns or adding an LLM fallback.",
            prompt, self.min_confidence, candidates
        )))
    }

    /// Get all registered domains.
    pub fn get_domains(&self) -> Vec<RegisteredDomain> {
        self.domains.read().clone()
    }

    /// Get a specific domain by name.
    pub fn get_domain(&self, name: &str) -> Option<RegisteredDomain> {
        self.domains
            .read()
            .iter()
            .find(|d| d.name == name)
            .cloned()
    }

    /// Update minimum confidence threshold.
    pub fn set_min_confidence(&mut self, threshold: f64) {
        self.min_confidence = threshold.clamp(0.0, 1.0);
    }
}

fn build_flat_pattern_list(domains: &[RegisteredDomain]) -> Vec<(String, String)> {
    domains
        .iter()
        .flat_map(|d| {
            d.patterns
                .iter()
                .map(move |p| (d.name.clone(), p.clone()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_router() -> IntentRouter {
        let router = IntentRouter::new(0.5);
        router
    }

    #[test]
    fn test_register_and_route_regex() {
        let router = test_router();
        router
            .register_domain(RegisteredDomain {
                name: "general".to_string(),
                patterns: vec![
                    r"hello".to_string(),
                    r"hi".to_string(),
                    r"what can you do".to_string(),
                ],
                agents: vec!["general_assistant".to_string()],
            })
            .unwrap();

        router
            .register_domain(RegisteredDomain {
                name: "engineering".to_string(),
                patterns: vec![
                    r"code".to_string(),
                    r"review".to_string(),
                    r"bug".to_string(),
                    r"refactor".to_string(),
                    r"deploy".to_string(),
                ],
                agents: vec![
                    "code_reviewer".to_string(),
                    "deployment_agent".to_string(),
                ],
            })
            .unwrap();

        // Test regex matching
        let result = router.route("can you review this code for me").unwrap();
        assert_eq!(result.domain, "engineering");
        assert!(result.confidence >= 0.5);
        assert!(!result.llm_fallback_used);

        let result = router.route("hello there").unwrap();
        assert_eq!(result.domain, "general");
    }

    #[test]
    fn test_low_confidence_error() {
        let router = test_router();
        router
            .register_domain(RegisteredDomain {
                name: "general".to_string(),
                patterns: vec!["hello".to_string()],
                agents: vec!["general_assistant".to_string()],
            })
            .unwrap();

        let result = router.route("quantum physics question");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("LowConfidence"));
    }

    #[test]
    fn test_cosine_similarity() {
        let vector = IntentVector::new(
            "test",
            &["code review".to_string(), "fix bugs".to_string()],
        );
        let query_terms = compute_query_terms("review my code");
        let sim = vector.cosine_similarity(&query_terms);
        assert!(sim > 0.0);
    }

    #[test]
    fn test_domain_registration_prevents_duplicates() {
        let router = test_router();
        router
            .register_domain(RegisteredDomain {
                name: "general".to_string(),
                patterns: vec!["hello".to_string()],
                agents: vec![],
            })
            .unwrap();

        let result = router.register_domain(RegisteredDomain {
            name: "general".to_string(),
            patterns: vec!["hi".to_string()],
            agents: vec![],
        });
        assert!(result.is_err());
    }
}
