use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id:                          String,
    pub prompt_price_per_token:      f64,
    pub completion_price_per_token:  f64,
}

impl ModelInfo {
    pub fn estimate_cost(&self, prompt_tokens: u32, completion_tokens: u32) -> f64 {
        prompt_tokens as f64     * self.prompt_price_per_token
            + completion_tokens as f64 * self.completion_price_per_token
    }
}

pub struct ModelFetcher {
    http:     reqwest::Client,
    base_url: String,
    api_key:  String,
}

impl ModelFetcher {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            http:     reqwest::Client::new(),
            base_url: base_url.into(),
            api_key:  api_key.into(),
        }
    }

    pub async fn fetch(&self) -> Result<Vec<ModelInfo>> {
        #[derive(Deserialize)]
        struct ApiResp { data: Vec<ApiModel> }

        #[derive(Deserialize)]
        struct ApiModel {
            id:      String,
            pricing: Option<Pricing>,
        }

        #[derive(Deserialize)]
        struct Pricing {
            prompt:     String,
            completion: String,
        }

        let resp: ApiResp = self.http
            .get(format!("{}/models", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.data.into_iter().map(|m| {
            let (p, c) = m.pricing.map(|pr| (
                pr.prompt.parse::<f64>().unwrap_or(0.0),
                pr.completion.parse::<f64>().unwrap_or(0.0),
            )).unwrap_or((0.0, 0.0));
            ModelInfo {
                id: m.id,
                prompt_price_per_token:     p,
                completion_price_per_token: c,
            }
        }).collect())
    }
}

pub type PricingCache = HashMap<String, ModelInfo>;

pub fn build_cache(models: Vec<ModelInfo>) -> PricingCache {
    models.into_iter().map(|m| (m.id.clone(), m)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn lists_models_from_api() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "data": [
                {
                    "id": "anthropic/claude-sonnet-4-6",
                    "pricing": { "prompt": "0.000003", "completion": "0.000015" }
                },
                {
                    "id": "openai/gpt-4o",
                    "pricing": { "prompt": "0.000005", "completion": "0.000015" }
                }
            ]
        });
        Mock::given(method("GET")).and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&server).await;

        let fetcher = ModelFetcher::new(server.uri(), "sk-test".to_string());
        let models  = fetcher.fetch().await.unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "anthropic/claude-sonnet-4-6");
        assert!((models[0].prompt_price_per_token - 0.000003).abs() < 1e-10);
    }

    #[test]
    fn cost_estimate_is_correct() {
        let info = ModelInfo {
            id: "x".into(),
            prompt_price_per_token:     0.000003,
            completion_price_per_token: 0.000015,
        };
        // 100 prompt tokens * 0.000003 + 50 completion tokens * 0.000015
        // = 0.0003 + 0.00075 = 0.00105
        let cost = info.estimate_cost(100, 50);
        assert!((cost - 0.00105).abs() < 1e-8);
    }

    #[test]
    fn build_cache_indexes_by_id() {
        let models = vec![
            ModelInfo { id: "a".into(), prompt_price_per_token: 0.001, completion_price_per_token: 0.002 },
            ModelInfo { id: "b".into(), prompt_price_per_token: 0.003, completion_price_per_token: 0.004 },
        ];
        let cache = build_cache(models);
        assert!(cache.contains_key("a"));
        assert!(cache.contains_key("b"));
        assert!((cache["a"].prompt_price_per_token - 0.001).abs() < 1e-10);
    }
}
