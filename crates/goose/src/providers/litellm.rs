use anyhow::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;
use goose_providers::cache_semantics::apply_chat_payload_breakpoints;
use goose_providers::errors::ProviderError;
use goose_providers::images::ImageFormat;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::api_client::{ApiClient, AuthMethod};
use super::base::{
    ConfigKey, MessageStream, ModelInfo, Provider, ProviderDef, ProviderMetadata,
    DEFAULT_PROVIDER_TIMEOUT_SECS,
};
use super::openai_compatible::{handle_status, stream_openai_compat};
use super::retry::ProviderRetry;
use crate::conversation::message::Message;
use goose_providers::model::ModelConfig;
use goose_providers::request_log::{start_log, LoggerHandleExt};
use rmcp::model::Tool;

const LITELLM_PROVIDER_NAME: &str = "litellm";
const LITELLM_DEFAULT_HOST: &str = "http://localhost:4000";
// CodyNo model IDs are account-specific and must be discovered from the
// authenticated gateway instead of hardcoding a generic OpenAI model.
pub const LITELLM_DEFAULT_MODEL: &str = "";
pub const LITELLM_DOC_URL: &str = "https://codyno.dev";

const MODEL_INFO_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);
const MODEL_INFO_FAILURE_TTL: Duration = Duration::from_secs(60);

#[derive(Debug)]
enum CachedModelInfo {
    Success(Vec<ModelInfo>),
    Failure(Instant),
}

#[derive(Debug, serde::Serialize)]
pub struct LiteLLMProvider {
    #[serde(skip)]
    api_client: ApiClient,
    base_path: String,
    #[serde(skip)]
    name: String,
    #[serde(skip)]
    cached_model_info: tokio::sync::Mutex<Option<CachedModelInfo>>,
}

impl LiteLLMProvider {
    pub async fn from_env(
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> Result<Self> {
        let config = crate::config::Config::global();
        let secrets = config
            .get_secrets("LITELLM_API_KEY", &["LITELLM_CUSTOM_HEADERS"])
            .unwrap_or_default();
        let api_key = secrets.get("LITELLM_API_KEY").cloned().unwrap_or_default();
        let host: String = config
            .get_param("LITELLM_HOST")
            .unwrap_or_else(|_| LITELLM_DEFAULT_HOST.to_string());
        let base_path: String = config
            .get_param("LITELLM_BASE_PATH")
            .unwrap_or_else(|_| "v1/chat/completions".to_string());
        let custom_headers: Option<HashMap<String, String>> = secrets
            .get("LITELLM_CUSTOM_HEADERS")
            .cloned()
            .map(parse_custom_headers);
        let timeout_secs: u64 = config
            .get_param("LITELLM_TIMEOUT")
            .unwrap_or(DEFAULT_PROVIDER_TIMEOUT_SECS);

        let auth = if api_key.is_empty() {
            AuthMethod::NoAuth
        } else {
            AuthMethod::BearerToken(api_key)
        };

        let mut api_client = ApiClient::with_timeout_and_tls(
            host,
            auth,
            std::time::Duration::from_secs(timeout_secs),
            tls_config,
        )?
        .with_request_builder(crate::session_context::session_id_request_builder());

        if let Some(headers) = custom_headers {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (key, value) in headers {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
                let header_value = reqwest::header::HeaderValue::from_str(&value)?;
                header_map.insert(header_name, header_value);
            }
            api_client = api_client.with_headers(header_map)?;
        }

        Ok(Self {
            api_client,
            base_path,
            name: LITELLM_PROVIDER_NAME.to_string(),
            cached_model_info: tokio::sync::Mutex::new(None),
        })
    }

    async fn get_or_fetch_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let mut cache = self.cached_model_info.lock().await;
        match cache.as_ref() {
            Some(CachedModelInfo::Success(models)) => return Ok(models.clone()),
            Some(CachedModelInfo::Failure(fetched_at))
                if fetched_at.elapsed() < MODEL_INFO_FAILURE_TTL =>
            {
                return Err(ProviderError::RequestFailed(
                    "LiteLLM model metadata is unavailable".to_string(),
                ));
            }
            Some(CachedModelInfo::Failure(_)) | None => {}
        }

        match tokio::time::timeout(MODEL_INFO_DISCOVERY_TIMEOUT, self.fetch_models_from_api()).await
        {
            Ok(Ok(models)) => {
                *cache = Some(CachedModelInfo::Success(models.clone()));
                Ok(models)
            }
            Ok(Err(error)) => {
                *cache = Some(CachedModelInfo::Failure(Instant::now()));
                Err(error)
            }
            Err(_) => {
                *cache = Some(CachedModelInfo::Failure(Instant::now()));
                Err(ProviderError::RequestFailed(
                    "LiteLLM model metadata discovery timed out".to_string(),
                ))
            }
        }
    }

    async fn fetch_models_from_api(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        // `/v1/models` is the authenticated, per-session catalog. The
        // administrative `/model/info` endpoint contains the full gateway
        // catalog and would expose models the CodyNo account cannot use.
        let response = self.api_client.request("v1/models").response_get().await?;

        if !response.status().is_success() {
            return Err(ProviderError::RequestFailed(format!(
                "Models endpoint returned status: {}",
                response.status()
            )));
        }

        let response_json: Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to parse models response: {}", e))
        })?;

        let models_data = response_json["data"].as_array().ok_or_else(|| {
            ProviderError::RequestFailed(
                "CodyNo models response is missing its data field".to_string(),
            )
        })?;

        let mut models = Vec::new();
        for model_data in models_data {
            if let Some(model_name) = model_data["id"].as_str() {
                models.push(ModelInfo::new(model_name));
            }
        }

        Ok(models)
    }

    async fn supports_cache_control(&self, model: &ModelConfig) -> bool {
        if let Ok(models) = self.get_or_fetch_models().await {
            if let Some(model_info) = models.iter().find(|m| m.name == model.model_name) {
                return model_info.supports_cache_control.unwrap_or(false);
            }
        }

        model.model_name.to_lowercase().contains("claude")
    }
}

impl goose_providers::base::ProviderDescriptor for LiteLLMProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            LITELLM_PROVIDER_NAME,
            "CodyNo",
            "CodyNo gateway for secure access to the available AI models",
            LITELLM_DEFAULT_MODEL,
            vec![],
            LITELLM_DOC_URL,
            vec![
                ConfigKey::new("LITELLM_API_KEY", true, true, None, true),
                ConfigKey::new(
                    "LITELLM_HOST",
                    true,
                    false,
                    Some(LITELLM_DEFAULT_HOST),
                    true,
                ),
                ConfigKey::new(
                    "LITELLM_BASE_PATH",
                    true,
                    false,
                    Some("v1/chat/completions"),
                    false,
                ),
                ConfigKey::new("LITELLM_CUSTOM_HEADERS", false, true, None, false),
                ConfigKey::new("LITELLM_TIMEOUT", false, false, Some("600"), false),
            ],
        )
        .with_setup(
            crate::providers::catalog::ProviderSetupMetadata::new(
                crate::providers::catalog::ProviderSetupCategory::Model,
                crate::providers::catalog::ProviderSetupMethod::ConfigFields,
                crate::providers::catalog::ProviderSetupGroup::Additional,
            )
            .with_field(
                "LITELLM_HOST",
                "CodyNo gateway",
                Some("https://models.codyno.dev"),
                None,
            )
            .with_field(
                "LITELLM_API_KEY",
                "CodyNo session",
                Some("Sign in through CodyNo"),
                None,
            ),
        )
    }
}

impl ProviderDef for LiteLLMProvider {
    type Provider = Self;

    fn from_env(
        _extensions: Vec<crate::config::ExtensionConfig>,
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(tls_config))
    }
}

#[async_trait]
impl Provider for LiteLLMProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    async fn get_context_limit(&self, model: &str, override_limit: Option<usize>) -> usize {
        goose_providers::context_limit::ContextLimitResolver::new(&self.name)
            .resolve(model, override_limit, || async {
                Ok(self
                    .get_or_fetch_models()
                    .await?
                    .iter()
                    .find(|info| info.name == model)
                    .and_then(|info| info.context_limit))
            })
            .await
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = goose_providers::formats::openai::create_request(
            model_config,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            true,
        )?;

        if !model_config.prompt_cache_disabled() && self.supports_cache_control(model_config).await
        {
            apply_chat_payload_breakpoints(&mut payload);
        }

        let mut log = start_log(model_config, &payload)?;
        let response = self
            .with_retry(|| async {
                handle_status(
                    self.api_client
                        .request(&self.base_path)
                        .model_headers(model_config)?
                        .streaming(true)
                        .response_post(&payload)
                        .await?,
                )
                .await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        stream_openai_compat(response, log)
    }

    fn skip_canonical_filtering(&self) -> bool {
        true
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let models = self.get_or_fetch_models().await?;
        Ok(models.iter().map(|m| m.name.clone()).collect())
    }
}

fn parse_custom_headers(headers_str: String) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for line in headers_str.lines() {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    headers
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn context_limit_negative_caches_failed_model_info() {
        let provider = LiteLLMProvider {
            api_client: ApiClient::new_with_tls(
                "http://127.0.0.1:1".to_string(),
                AuthMethod::NoAuth,
                None,
            )
            .unwrap(),
            base_path: "v1/chat/completions".to_string(),
            name: LITELLM_PROVIDER_NAME.to_string(),
            cached_model_info: tokio::sync::Mutex::new(None),
        };

        assert_eq!(
            provider.get_context_limit("unknown-model", None).await,
            goose_providers::model::DEFAULT_CONTEXT_LIMIT
        );
        assert!(matches!(
            provider.cached_model_info.lock().await.as_ref(),
            Some(CachedModelInfo::Failure(_))
        ));
        assert_eq!(
            provider.get_context_limit("unknown-model", None).await,
            goose_providers::model::DEFAULT_CONTEXT_LIMIT
        );
    }

    #[tokio::test]
    async fn expired_failure_allows_model_info_retry() {
        let provider = LiteLLMProvider {
            api_client: ApiClient::new_with_tls(
                "http://127.0.0.1:1".to_string(),
                AuthMethod::NoAuth,
                None,
            )
            .unwrap(),
            base_path: "v1/chat/completions".to_string(),
            name: LITELLM_PROVIDER_NAME.to_string(),
            cached_model_info: tokio::sync::Mutex::new(Some(CachedModelInfo::Failure(
                Instant::now() - MODEL_INFO_FAILURE_TTL,
            ))),
        };

        assert!(provider.get_or_fetch_models().await.is_err());
        assert!(matches!(
            provider.cached_model_info.lock().await.as_ref(),
            Some(CachedModelInfo::Failure(fetched_at))
                if fetched_at.elapsed() < MODEL_INFO_FAILURE_TTL
        ));
    }

    #[tokio::test]
    async fn context_limit_uses_cached_model_info() {
        let cached_model_info =
            tokio::sync::Mutex::new(Some(CachedModelInfo::Success(vec![ModelInfo::new(
                "cached-model",
            )
            .with_context_limit(32_000)])));
        let provider = LiteLLMProvider {
            api_client: ApiClient::new_with_tls(
                "http://127.0.0.1:1".to_string(),
                AuthMethod::NoAuth,
                None,
            )
            .unwrap(),
            base_path: "v1/chat/completions".to_string(),
            name: LITELLM_PROVIDER_NAME.to_string(),
            cached_model_info,
        };

        assert_eq!(
            provider.get_context_limit("cached-model", None).await,
            32_000
        );
    }

    #[tokio::test]
    async fn stream_requests_sse_and_forwards_multiple_events() {
        let server = MockServer::start().await;
        let sse_body = concat!(
            "data: {\"id\":\"chatcmpl-test\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"Hello\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"chatcmpl-test\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" CodyNo\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"chatcmpl-test\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2,\"total_tokens\":3}}\n\n",
            "data: [DONE]\n\n"
        );

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(body_partial_json(json!({
                "model": "stream-test-model",
                "stream": true
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(sse_body),
            )
            .expect(1)
            .mount(&server)
            .await;

        let provider = LiteLLMProvider {
            api_client: ApiClient::new_with_tls(server.uri(), AuthMethod::NoAuth, None).unwrap(),
            base_path: "v1/chat/completions".to_string(),
            name: LITELLM_PROVIDER_NAME.to_string(),
            cached_model_info: tokio::sync::Mutex::new(None),
        };

        let mut stream = provider
            .stream(
                &ModelConfig::new("stream-test-model"),
                "You are CodyNo.",
                &[Message::user().with_text("Say hello")],
                &[],
            )
            .await
            .unwrap();

        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event.unwrap());
        }

        assert!(events.len() >= 2, "expected incremental SSE events");
    }
}
