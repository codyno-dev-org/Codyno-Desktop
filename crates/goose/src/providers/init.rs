use std::path::PathBuf;
use std::sync::{Arc, RwLock};

// CodyNo: only the LiteLLM provider is imported — every other provider import
// went with the registrations in init_registry().
use super::{
    base::{Provider, ProviderMetadata},
    litellm::LiteLLMProvider,
    provider_registry::ProviderRegistry,
};
use crate::config::ExtensionConfig;
use crate::providers::base::ProviderType;
use crate::{
    providers::provider_registry::ProviderEntry,
};
use anyhow::Result;
use tokio::sync::OnceCell;

static REGISTRY: OnceCell<RwLock<ProviderRegistry>> = OnceCell::const_new();

async fn init_registry() -> RwLock<ProviderRegistry> {
    let tls_config =
        crate::config::tls::provider_tls_config_from_config(crate::config::Config::global())
            .expect("failed to load provider TLS config");
    // CodyNo: the agent is locked to the CodyNo gateway. Only the LiteLLM
    // provider is registered, so no BYOK provider can be selected, configured or
    // reached — the model catalogue a user sees is exactly what our gateway
    // serves them for their plan. Upstream registers ~33 providers here; keeping
    // that list would let anyone bypass our billing by pointing the agent at
    // their own key.
    let registry = ProviderRegistry::new(tls_config).with_providers(|registry| {
        use super::inventory::registrations;

        registry.register_with_inventory::<LiteLLMProvider>(
            true,
            Some(registrations::refresh_only().with_configured(|| {
                let config = crate::config::Config::global();
                config
                    .get_param::<serde_json::Value>("LITELLM_HOST")
                    .is_ok()
                    || config
                        .get_secret::<serde_json::Value>("LITELLM_API_KEY")
                        .is_ok()
            })),
        );
    });

    // No set_cleanup registrations: every provider that cached credential state
    // (github_copilot, databricks, kimi_code, chatgpt_codex, gemini_oauth,
    // xai_oauth, huggingface) is gone.
    //
    // load_custom_providers_into_registry() is deliberately not called. It reads
    // ~/.config/goose/custom_providers/*.json, which would let a user declare an
    // arbitrary OpenAI-compatible endpoint and re-open exactly the hole the lock
    // above closes.

    RwLock::new(registry)
}

async fn get_registry() -> &'static RwLock<ProviderRegistry> {
    REGISTRY.get_or_init(init_registry).await
}

pub async fn providers() -> Vec<(ProviderMetadata, ProviderType)> {
    get_registry()
        .await
        .read()
        .unwrap()
        .all_metadata_with_types()
}

pub async fn refresh_custom_providers() -> Result<()> {
    // CodyNo: custom providers are disabled. Kept as a no-op so the callers that
    // poll it (settings UI, config watcher) still compile and simply observe an
    // unchanged registry.
    Ok(())
}

pub async fn get_from_registry(name: &str) -> Result<ProviderEntry> {
    let guard = get_registry().await.read().unwrap();
    guard
        .entries
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", name))
        .cloned()
}

pub async fn inventory_identity(name: &str) -> Result<super::inventory::InventoryIdentityInput> {
    get_from_registry(name).await?.inventory_identity()
}

pub async fn create(name: &str, extensions: Vec<ExtensionConfig>) -> Result<Arc<dyn Provider>> {
    let entry = get_from_registry(name).await?;
    entry.create(extensions).await
}

pub async fn create_with_working_dir(
    name: &str,
    extensions: Vec<ExtensionConfig>,
    working_dir: PathBuf,
) -> Result<Arc<dyn Provider>> {
    let entry = get_from_registry(name).await?;
    entry.create_with_working_dir(extensions, working_dir).await
}

pub async fn create_with_default_model(
    name: impl AsRef<str>,
    extensions: Vec<ExtensionConfig>,
) -> Result<Arc<dyn Provider>> {
    get_from_registry(name.as_ref())
        .await?
        .create_with_default_model(extensions)
        .await
}

pub async fn cleanup_provider(name: &str) -> Result<()> {
    let cleanup_fn = {
        let registry = get_registry().await.read().unwrap();
        registry
            .entries
            .get(name)
            .and_then(|entry| entry.cleanup.clone())
    };
    if let Some(cleanup) = cleanup_fn {
        return cleanup().await;
    }
    Ok(())
}

pub async fn create_with_named_model(
    provider_name: &str,
    extensions: Vec<ExtensionConfig>,
) -> Result<Arc<dyn Provider>> {
    create(provider_name, extensions).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::paths::Paths;
    use std::fs;

    #[tokio::test]
    async fn test_huggingface_provider_registry_wiring() {
        let huggingface = get_from_registry("huggingface")
            .await
            .expect("huggingface provider should be registered");
        let meta = huggingface.metadata();

        assert_eq!(huggingface.provider_type(), ProviderType::Preferred);
        assert_eq!(meta.display_name, "Hugging Face");
        assert_eq!(meta.default_model, "Qwen/Qwen3-Coder-480B-A35B-Instruct");
        assert!(meta
            .config_keys
            .iter()
            .any(|key| key.name == "HF_TOKEN" && key.secret));
    }

    #[tokio::test]
    async fn test_aimlapi_provider_registry_wiring() {
        let aimlapi = get_from_registry("aimlapi")
            .await
            .expect("aimlapi provider should be registered");
        let meta = aimlapi.metadata();

        assert_eq!(meta.name, "aimlapi");
        assert_eq!(meta.display_name, "AI/ML API");
        assert_eq!(meta.default_model, "openai/gpt-5-5");
        assert!(meta
            .config_keys
            .iter()
            .any(|key| key.name == "AIMLAPI_API_KEY" && key.secret));
    }

    #[tokio::test]
    async fn test_gondola_provider_registry_wiring() {
        let gondola = get_from_registry("gondola")
            .await
            .expect("gondola provider should be registered");
        let meta = gondola.metadata();

        assert_eq!(meta.name, "gondola");
        assert_eq!(meta.default_model, "deepseek-v4-flash");
        assert!(meta
            .config_keys
            .iter()
            .any(|key| key.name == "GONDOLA_API_KEY" && key.secret));
    }

    #[tokio::test]
    async fn test_openai_compatible_providers_config_keys() {
        let providers_list = providers().await;
        let required_api_key_cases = vec![
            ("groq", "GROQ_API_KEY"),
            ("mistral", "MISTRAL_API_KEY"),
            ("custom_deepseek", "DEEPSEEK_API_KEY"),
        ];
        for (name, expected_key) in required_api_key_cases {
            if let Some((meta, _)) = providers_list.iter().find(|(m, _)| m.name == name) {
                assert!(
                    !meta.config_keys.is_empty(),
                    "{name} provider should have config keys"
                );
                assert_eq!(
                    meta.config_keys[0].name, expected_key,
                    "First config key for {name} should be {expected_key}, got {}",
                    meta.config_keys[0].name
                );
                assert!(
                    meta.config_keys[0].required,
                    "{expected_key} should be required"
                );
                assert!(
                    meta.config_keys[0].secret,
                    "{expected_key} should be secret"
                );
            } else {
                // Provider not registered; skip test for this provider
                continue;
            }
        }

        if let Some((meta, _)) = providers_list.iter().find(|(m, _)| m.name == "openai") {
            assert!(
                !meta.config_keys.is_empty(),
                "openai provider should have config keys"
            );
            assert_eq!(
                meta.config_keys[0].name, "OPENAI_API_KEY",
                "First config key for openai should be OPENAI_API_KEY"
            );
            assert!(
                !meta.config_keys[0].required,
                "OPENAI_API_KEY should be optional for local server support"
            );
            assert!(
                meta.config_keys[0].secret,
                "OPENAI_API_KEY should be secret"
            );
        }
    }

    #[tokio::test]
    async fn test_goose_context_limit_overrides_known_models_and_defaults() {
        let _guard = env_lock::lock_env([
            ("GOOSE_PATH_ROOT", None::<&str>),
            ("GOOSE_CONTEXT_LIMIT", Some("1000000")),
            ("GOOSE_MAX_TOKENS", None::<&str>),
            ("GOOSE_TEMPERATURE", None::<&str>),
            ("GOOSE_TOOLSHIM", None::<&str>),
            ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
            ("GOOSE_THINKING_EFFORT", None::<&str>),
        ]);

        let openai = get_from_registry("openai")
            .await
            .expect("openai provider should be registered");
        let openai_provider = openai
            .create(vec![])
            .await
            .expect("openai provider should be created");
        assert_eq!(
            openai_provider
                .get_context_limit("totally-unknown-model", Some(1_000_000))
                .await,
            1_000_000
        );

        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        std::env::set_var("GOOSE_PATH_ROOT", temp_dir.path());

        let custom_dir = Paths::config_dir().join("custom_providers");
        fs::create_dir_all(&custom_dir).expect("custom providers dir should be created");

        let custom_inf = r#"{
  "name": "custom_inf",
  "engine": "openai",
  "display_name": "Custom Inf",
  "description": "test provider",
  "api_key_env": "",
  "base_url": "https://example.invalid/v1/chat/completions",
  "models": [
    {"name": "kimi-k2.5", "context_limit": 256000}
  ],
  "requires_auth": false
}"#;
        fs::write(custom_dir.join("custom_inf.json"), custom_inf)
            .expect("custom_inf.json should be written");

        refresh_custom_providers()
            .await
            .expect("custom providers should refresh");

        let inf_entry = get_from_registry("custom_inf")
            .await
            .expect("custom_inf entry should exist");
        let inf_provider = inf_entry
            .create(vec![])
            .await
            .expect("custom_inf provider should be created");
        assert_eq!(
            inf_provider
                .get_context_limit("kimi-k2.5", Some(1_000_000))
                .await,
            1_000_000
        );

        std::env::remove_var("GOOSE_PATH_ROOT");
    }

    #[tokio::test]
    async fn test_litellm_supports_inventory_refresh() {
        let entry = get_from_registry("litellm")
            .await
            .expect("litellm should be registered");
        assert!(
            entry.supports_inventory_refresh(),
            "litellm must support inventory refresh so the model picker calls fetch_supported_models"
        );
    }

    #[tokio::test]
    async fn test_api_backed_model_providers_are_registered_for_refresh() {
        for provider_name in [
            "gcp_vertex_ai",
            "github_copilot",
            "kimi_code",
            "nano-gpt",
            "tetrate",
            "xai",
            "xai_oauth",
        ] {
            let entry = get_from_registry(provider_name)
                .await
                .expect("dynamic model provider should be registered");
            assert!(
                entry.supports_inventory_refresh(),
                "{provider_name} must refresh its model inventory"
            );
        }
    }

    #[tokio::test]
    async fn test_litellm_configured_without_api_key() {
        let _guard = env_lock::lock_env([
            ("LITELLM_API_KEY", None::<&str>),
            ("LITELLM_HOST", Some("http://localhost:4000")),
        ]);

        let entry = get_from_registry("litellm")
            .await
            .expect("litellm should be registered");
        assert!(
            entry.inventory_configured(),
            "litellm should be considered configured when LITELLM_HOST is set without an API key"
        );
    }

    #[tokio::test]
    async fn test_litellm_not_configured_without_any_settings() {
        let _guard = env_lock::lock_env([
            ("LITELLM_API_KEY", None::<&str>),
            ("LITELLM_HOST", None::<&str>),
        ]);

        let entry = get_from_registry("litellm")
            .await
            .expect("litellm should be registered");
        assert!(
            !entry.inventory_configured(),
            "litellm should not be considered configured when no settings are present"
        );
    }
}
