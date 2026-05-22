use std::time::Duration;

use codex_api::ModelsClient;
use codex_api::ReqwestTransport;
use codex_api::map_api_error;
use codex_app_server_protocol::Model;
use codex_app_server_protocol::ModelServiceTier;
use codex_app_server_protocol::ModelUpgradeInfo;
use codex_app_server_protocol::ReasoningEffortOption;
use codex_core::ThreadManager;
use codex_core::config::Config;
use codex_login::AuthManager;
use codex_login::default_client::build_reqwest_client;
use codex_model_provider::create_model_provider;
use codex_models_manager::client_version_to_whole;
use codex_models_manager::manager::ModelsManager;
use codex_models_manager::manager::RefreshStrategy;
use codex_models_manager::manager::StaticModelsManager;
use codex_protocol::openai_models::ModelPreset;
use codex_protocol::openai_models::ModelsResponse;
use codex_protocol::openai_models::ReasoningEffortPreset;
use reqwest::header::HeaderMap;
use std::sync::Arc;
use tokio::time::timeout;

const PROVIDER_MODELS_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn supported_models(
    thread_manager: Arc<ThreadManager>,
    include_hidden: bool,
) -> Vec<Model> {
    thread_manager
        .list_models(RefreshStrategy::OnlineIfUncached)
        .await
        .into_iter()
        .filter(|preset| include_hidden || preset.show_in_picker)
        .map(model_from_preset)
        .collect()
}

pub async fn supported_models_for_provider(
    config: &Config,
    auth_manager: Arc<AuthManager>,
    provider_id: &str,
    include_hidden: bool,
) -> Result<Vec<Model>, String> {
    let provider_info = config
        .model_providers
        .get(provider_id)
        .cloned()
        .ok_or_else(|| format!("unknown provider id: {provider_id}"))?;
    let provider = create_model_provider(provider_info, Some(auth_manager));
    let presets = if provider.info().is_amazon_bedrock() {
        provider
            .models_manager(
                config.codex_home.to_path_buf(),
                /*config_model_catalog*/ None,
            )
            .list_models(RefreshStrategy::OnlineIfUncached)
            .await
    } else {
        let remote_models = fetch_provider_catalog_models(provider.as_ref()).await?;
        StaticModelsManager::new(
            provider.auth_manager(),
            ModelsResponse {
                models: remote_models,
            },
        )
        .list_models(RefreshStrategy::OnlineIfUncached)
        .await
    };
    Ok(presets
        .into_iter()
        .filter(|preset| include_hidden || preset.show_in_picker)
        .map(model_from_preset)
        .collect())
}

async fn fetch_provider_catalog_models(
    provider: &dyn codex_model_provider::ModelProvider,
) -> Result<Vec<codex_protocol::openai_models::ModelInfo>, String> {
    let api_provider = provider
        .api_provider()
        .await
        .map_err(|err| err.to_string())?;
    let api_auth = provider.api_auth().await.map_err(|err| err.to_string())?;
    let client_version = client_version_to_whole();
    let transport = ReqwestTransport::new(build_reqwest_client());
    let client = ModelsClient::new(transport, api_provider, api_auth);
    let (models, _) = timeout(
        PROVIDER_MODELS_TIMEOUT,
        client.list_models(&client_version, HeaderMap::new()),
    )
    .await
    .map_err(|_| "provider model catalog request timed out".to_string())?
    .map_err(map_api_error)
    .map_err(|err| err.to_string())?;
    Ok(models)
}

fn model_from_preset(preset: ModelPreset) -> Model {
    Model {
        id: preset.id.to_string(),
        model: preset.model.to_string(),
        upgrade: preset.upgrade.as_ref().map(|upgrade| upgrade.id.clone()),
        upgrade_info: preset.upgrade.as_ref().map(|upgrade| ModelUpgradeInfo {
            model: upgrade.id.clone(),
            upgrade_copy: upgrade.upgrade_copy.clone(),
            model_link: upgrade.model_link.clone(),
            migration_markdown: upgrade.migration_markdown.clone(),
        }),
        availability_nux: preset.availability_nux.map(Into::into),
        display_name: preset.display_name.to_string(),
        description: preset.description.to_string(),
        hidden: !preset.show_in_picker,
        supported_reasoning_efforts: reasoning_efforts_from_preset(
            preset.supported_reasoning_efforts,
        ),
        default_reasoning_effort: preset.default_reasoning_effort,
        input_modalities: preset.input_modalities,
        supports_personality: preset.supports_personality,
        additional_speed_tiers: preset.additional_speed_tiers,
        service_tiers: preset
            .service_tiers
            .into_iter()
            .map(|service_tier| ModelServiceTier {
                id: service_tier.id,
                name: service_tier.name,
                description: service_tier.description,
            })
            .collect(),
        is_default: preset.is_default,
    }
}

fn reasoning_efforts_from_preset(
    efforts: Vec<ReasoningEffortPreset>,
) -> Vec<ReasoningEffortOption> {
    efforts
        .iter()
        .map(|preset| ReasoningEffortOption {
            reasoning_effort: preset.effort,
            description: preset.description.to_string(),
        })
        .collect()
}
