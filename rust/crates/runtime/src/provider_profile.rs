use std::fmt::{Display, Formatter};

use crate::config::RuntimeConfig;
use crate::json::JsonValue;

const PRIMARY_PROFILE_ENV: &str = "SAICODE_PROFILE";
const PRIMARY_MODEL_ENV: &str = "SAICODE_MODEL";
const PRIMARY_BASE_URL_ENV: &str = "SAICODE_BASE_URL";
const PRIMARY_API_KEY_ENV: &str = "SAICODE_API_KEY";
const SHARED_ROUTER_MODEL: &str = "qwen/qwen3.5-122b-a10b";
const PROVIDER_MODEL_PREFIXES: &[&str] = &["cpa", "cliproxyapi", "nvidia", "opencode", "custom"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionSource {
    Cli,
    Env(&'static str),
    Config(&'static str),
    ProfileDefault,
    Missing,
}

impl ResolutionSource {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Env(name) => name,
            Self::Config(label) => label,
            Self::ProfileDefault => "profile-default",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    PrimaryEnv,
    ProfileEnv,
    ConfigValue,
    Missing,
}

impl CredentialSource {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::PrimaryEnv => PRIMARY_API_KEY_ENV,
            Self::ProfileEnv => "profile-env",
            Self::ConfigValue => "config-value",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProfile {
    pub name: String,
    pub base_url_env: String,
    pub base_url: String,
    pub api_key_env: String,
    pub default_model: String,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub request_timeout_ms: u64,
    pub max_retries: u32,
}

#[must_use]
pub fn builtin_profiles() -> Vec<ProviderProfile> {
    ["cpa", "cliproxyapi", "nvidia", "opencode", "custom"]
        .into_iter()
        .filter_map(builtin_profile)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialResolution {
    pub source: CredentialSource,
    pub env_name: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProviderProfile {
    pub profile_name: String,
    pub profile_source: ResolutionSource,
    pub model: String,
    pub model_source: ResolutionSource,
    pub base_url: Option<String>,
    pub base_url_source: ResolutionSource,
    pub credential: CredentialResolution,
    pub profile: ProviderProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderLaunchConfig {
    pub profile_name: String,
    pub provider_label: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub request_timeout_ms: u64,
    pub max_retries: u32,
    pub supports_tools: bool,
    pub supports_streaming: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProfileError {
    message: String,
}

impl ProviderProfileError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for ProviderProfileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ProviderProfileError {}

pub struct ProfileResolver;

impl ProfileResolver {
    pub fn resolve(
        runtime_config: &RuntimeConfig,
        cli_profile: Option<&str>,
        cli_model: Option<&str>,
    ) -> Result<ResolvedProviderProfile, ProviderProfileError> {
        let (profile_name, profile_source) =
            resolve_active_profile_name(runtime_config, cli_profile);
        resolve_named_profile(runtime_config, &profile_name, profile_source, cli_model)
    }

    pub fn resolve_named(
        runtime_config: &RuntimeConfig,
        profile_name: &str,
        cli_model: Option<&str>,
    ) -> Result<ResolvedProviderProfile, ProviderProfileError> {
        resolve_named_profile(
            runtime_config,
            profile_name,
            ResolutionSource::Cli,
            cli_model,
        )
    }

    #[must_use]
    pub fn available_profile_names(runtime_config: &RuntimeConfig) -> Vec<String> {
        let mut names = builtin_profiles()
            .into_iter()
            .map(|profile| profile.name)
            .collect::<Vec<_>>();
        if let Some(profiles) = runtime_config
            .get("profiles")
            .and_then(JsonValue::as_object)
        {
            for name in profiles.keys() {
                if !names.iter().any(|candidate| candidate == name) {
                    names.push(name.clone());
                }
            }
        }
        names.sort();
        names
    }
}

fn resolve_named_profile(
    runtime_config: &RuntimeConfig,
    profile_name: &str,
    profile_source: ResolutionSource,
    cli_model: Option<&str>,
) -> Result<ResolvedProviderProfile, ProviderProfileError> {
    let mut profile = builtin_profile(&profile_name)
        .or_else(|| {
            profile_block(runtime_config, profile_name).map(|_| custom_profile(profile_name))
        })
        .ok_or_else(|| {
            ProviderProfileError::new(format!("unsupported profile `{profile_name}`"))
        })?;
    apply_profile_overrides(runtime_config, &profile_name, &mut profile);

    let (model, model_source) = resolve_model(runtime_config, cli_model, &profile);
    let (base_url, base_url_source) = resolve_base_url(runtime_config, &profile);
    let credential = resolve_credential(runtime_config, &profile);

    Ok(ResolvedProviderProfile {
        profile_name: profile_name.to_string(),
        profile_source,
        model,
        model_source,
        base_url,
        base_url_source,
        credential,
        profile,
    })
}

fn custom_profile(name: &str) -> ProviderProfile {
    ProviderProfile {
        name: name.to_string(),
        base_url_env: PRIMARY_BASE_URL_ENV.to_string(),
        base_url: String::new(),
        api_key_env: PRIMARY_API_KEY_ENV.to_string(),
        default_model: SHARED_ROUTER_MODEL.to_string(),
        supports_tools: true,
        supports_streaming: true,
        request_timeout_ms: 120_000,
        max_retries: 2,
    }
}

pub struct CredentialResolver;

impl CredentialResolver {
    pub fn resolve(
        runtime_config: &RuntimeConfig,
        profile: &ProviderProfile,
    ) -> CredentialResolution {
        resolve_credential(runtime_config, profile)
    }
}

pub struct ProviderLauncher;

impl ProviderLauncher {
    pub fn prepare(
        resolved: &ResolvedProviderProfile,
    ) -> Result<ProviderLaunchConfig, ProviderProfileError> {
        let Some(base_url) = resolved
            .base_url
            .clone()
            .filter(|value| !value.trim().is_empty())
        else {
            return Err(ProviderProfileError::new(format!(
                "profile `{}` does not have a base URL; set `{}` or `base_url` in config",
                resolved.profile_name, PRIMARY_BASE_URL_ENV
            )));
        };
        let Some(api_key) = resolved
            .credential
            .api_key
            .clone()
            .filter(|value| !value.trim().is_empty())
        else {
            return Err(ProviderProfileError::new(format!(
                "profile `{}` does not have credentials; export `{}`",
                resolved.profile_name, resolved.credential.env_name
            )));
        };

        Ok(ProviderLaunchConfig {
            profile_name: resolved.profile_name.clone(),
            provider_label: resolved.profile_name.clone(),
            base_url,
            api_key,
            model: normalize_provider_model(&resolved.model),
            request_timeout_ms: resolved.profile.request_timeout_ms,
            max_retries: resolved.profile.max_retries,
            supports_tools: resolved.profile.supports_tools,
            supports_streaming: resolved.profile.supports_streaming,
        })
    }
}

fn resolve_active_profile_name(
    runtime_config: &RuntimeConfig,
    cli_profile: Option<&str>,
) -> (String, ResolutionSource) {
    if let Some(value) = cli_profile.and_then(non_empty) {
        return (value.to_string(), ResolutionSource::Cli);
    }
    if let Some(value) = read_non_empty_env(PRIMARY_PROFILE_ENV) {
        return (value, ResolutionSource::Env(PRIMARY_PROFILE_ENV));
    }
    if let Some(value) = merged_string(runtime_config, &["profile"]) {
        return (value, ResolutionSource::Config("config.profile"));
    }
    ("cliproxyapi".to_string(), ResolutionSource::ProfileDefault)
}

fn resolve_model(
    runtime_config: &RuntimeConfig,
    cli_model: Option<&str>,
    profile: &ProviderProfile,
) -> (String, ResolutionSource) {
    if let Some(value) = cli_model.and_then(non_empty) {
        return (value.to_string(), ResolutionSource::Cli);
    }
    if let Some(value) = read_non_empty_env(PRIMARY_MODEL_ENV) {
        return (value, ResolutionSource::Env(PRIMARY_MODEL_ENV));
    }
    if let Some(value) = merged_string(runtime_config, &["model"]) {
        return (value, ResolutionSource::Config("config.model"));
    }
    (
        profile.default_model.clone(),
        ResolutionSource::ProfileDefault,
    )
}

fn resolve_base_url(
    runtime_config: &RuntimeConfig,
    profile: &ProviderProfile,
) -> (Option<String>, ResolutionSource) {
    if let Some(value) = read_non_empty_env(PRIMARY_BASE_URL_ENV) {
        return (Some(value), ResolutionSource::Env(PRIMARY_BASE_URL_ENV));
    }
    if let Some(value) = read_non_empty_env(profile.base_url_env.as_str()) {
        return (
            Some(value),
            env_source(profile.base_url_env.as_str()).unwrap_or(ResolutionSource::ProfileDefault),
        );
    }
    if let Some(value) = merged_string(runtime_config, &["base_url", "baseUrl"]) {
        return (Some(value), ResolutionSource::Config("config.base_url"));
    }
    match non_empty(profile.base_url.as_str()) {
        Some(value) => (Some(value.to_string()), ResolutionSource::ProfileDefault),
        None => (None, ResolutionSource::Missing),
    }
}

fn resolve_credential(
    runtime_config: &RuntimeConfig,
    profile: &ProviderProfile,
) -> CredentialResolution {
    let configured_env_name = merged_string(runtime_config, &["api_key_env", "apiKeyEnv"])
        .unwrap_or_else(|| profile.api_key_env.clone());
    if let Some(api_key) = read_non_empty_env(PRIMARY_API_KEY_ENV) {
        return CredentialResolution {
            source: CredentialSource::PrimaryEnv,
            env_name: PRIMARY_API_KEY_ENV.to_string(),
            api_key: Some(api_key),
        };
    }
    if let Some(api_key) = read_non_empty_env(configured_env_name.as_str()) {
        return CredentialResolution {
            source: CredentialSource::ProfileEnv,
            env_name: configured_env_name.clone(),
            api_key: Some(api_key),
        };
    }
    if let Some(api_key) = fallback_profile_api_key(profile.name.as_str()) {
        return CredentialResolution {
            source: CredentialSource::ProfileEnv,
            env_name: fallback_env_name(profile.name.as_str()).to_string(),
            api_key: Some(api_key),
        };
    }
    if let Some(api_key) = profile_inline_api_key(runtime_config, profile.name.as_str()) {
        return CredentialResolution {
            source: CredentialSource::ConfigValue,
            env_name: "config.providers.*.apiKey".to_string(),
            api_key: Some(api_key),
        };
    }
    CredentialResolution {
        source: CredentialSource::Missing,
        env_name: configured_env_name,
        api_key: None,
    }
}

fn builtin_profile(name: &str) -> Option<ProviderProfile> {
    let lower = name.trim().to_ascii_lowercase();
    Some(match lower.as_str() {
        "cpa" => ProviderProfile {
            name: "cpa".to_string(),
            base_url_env: "CPA_BASE_URL".to_string(),
            base_url: String::new(),
            api_key_env: "CPA_API_KEY".to_string(),
            default_model: SHARED_ROUTER_MODEL.to_string(),
            supports_tools: true,
            supports_streaming: true,
            request_timeout_ms: 120_000,
            max_retries: 2,
        },
        "cliproxyapi" => ProviderProfile {
            name: "cliproxyapi".to_string(),
            base_url_env: "CLIPROXYAPI_BASE_URL".to_string(),
            base_url: String::new(),
            api_key_env: "CLIPROXYAPI_API_KEY".to_string(),
            default_model: SHARED_ROUTER_MODEL.to_string(),
            supports_tools: true,
            supports_streaming: true,
            request_timeout_ms: 120_000,
            max_retries: 2,
        },
        "nvidia" => ProviderProfile {
            name: "nvidia".to_string(),
            base_url_env: PRIMARY_BASE_URL_ENV.to_string(),
            base_url: String::new(),
            api_key_env: "NVIDIA_API_KEY".to_string(),
            default_model: "meta/llama-3.3-70b-instruct".to_string(),
            supports_tools: true,
            supports_streaming: true,
            request_timeout_ms: 120_000,
            max_retries: 2,
        },
        "opencode" => ProviderProfile {
            name: "opencode".to_string(),
            base_url_env: PRIMARY_BASE_URL_ENV.to_string(),
            base_url: String::new(),
            api_key_env: "OPENCODE_API_KEY".to_string(),
            default_model: SHARED_ROUTER_MODEL.to_string(),
            supports_tools: true,
            supports_streaming: true,
            request_timeout_ms: 120_000,
            max_retries: 2,
        },
        "custom" => ProviderProfile {
            name: "custom".to_string(),
            base_url_env: PRIMARY_BASE_URL_ENV.to_string(),
            base_url: String::new(),
            api_key_env: PRIMARY_API_KEY_ENV.to_string(),
            default_model: SHARED_ROUTER_MODEL.to_string(),
            supports_tools: true,
            supports_streaming: true,
            request_timeout_ms: 120_000,
            max_retries: 2,
        },
        _ => return None,
    })
}

fn apply_profile_overrides(
    runtime_config: &RuntimeConfig,
    profile_name: &str,
    profile: &mut ProviderProfile,
) {
    let Some(profile_block) = profile_block(runtime_config, profile_name) else {
        return;
    };

    if let Some(value) = object_string(profile_block, &["base_url", "baseUrl"]) {
        profile.base_url = value;
    }
    if let Some(value) = object_string(profile_block, &["api_key_env", "apiKeyEnv"]) {
        profile.api_key_env = value;
    }
    if let Some(value) = object_string(profile_block, &["default_model", "defaultModel", "model"]) {
        profile.default_model = value;
    }
    if let Some(value) = object_string(profile_block, &["base_url_env", "baseUrlEnv"]) {
        profile.base_url_env = value;
    }
    if let Some(value) = object_bool(profile_block, &["supports_tools", "supportsTools"]) {
        profile.supports_tools = value;
    }
    if let Some(value) = object_bool(profile_block, &["supports_streaming", "supportsStreaming"]) {
        profile.supports_streaming = value;
    }
    if let Some(value) = object_u64(profile_block, &["request_timeout_ms", "requestTimeoutMs"]) {
        profile.request_timeout_ms = value;
    }
    if let Some(value) = object_u64(profile_block, &["max_retries", "maxRetries"]) {
        profile.max_retries = value as u32;
    }
}

fn profile_block<'a>(
    runtime_config: &'a RuntimeConfig,
    profile_name: &str,
) -> Option<&'a std::collections::BTreeMap<String, JsonValue>> {
    runtime_config
        .get("profiles")
        .and_then(JsonValue::as_object)
        .and_then(|profiles| profiles.get(profile_name))
        .and_then(JsonValue::as_object)
        .or_else(|| {
            runtime_config
                .get("providers")
                .and_then(JsonValue::as_object)
                .and_then(|profiles| profiles.get(profile_name))
                .and_then(JsonValue::as_object)
        })
}

fn merged_string(runtime_config: &RuntimeConfig, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| runtime_config.get(key))
        .and_then(JsonValue::as_str)
        .and_then(non_empty)
        .map(ToOwned::to_owned)
}

fn object_string(
    object: &std::collections::BTreeMap<String, JsonValue>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key))
        .and_then(JsonValue::as_str)
        .and_then(non_empty)
        .map(ToOwned::to_owned)
}

fn object_bool(
    object: &std::collections::BTreeMap<String, JsonValue>,
    keys: &[&str],
) -> Option<bool> {
    keys.iter()
        .find_map(|key| object.get(*key))
        .and_then(JsonValue::as_bool)
}

fn object_u64(
    object: &std::collections::BTreeMap<String, JsonValue>,
    keys: &[&str],
) -> Option<u64> {
    keys.iter()
        .find_map(|key| object.get(*key))
        .and_then(JsonValue::as_i64)
        .and_then(|value| u64::try_from(value).ok())
}

fn read_non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .and_then(|value| non_empty(value.trim()).map(ToOwned::to_owned))
}

fn env_source(name: &str) -> Option<ResolutionSource> {
    match name {
        "CPA_BASE_URL" => Some(ResolutionSource::Env("CPA_BASE_URL")),
        "CLIPROXYAPI_BASE_URL" => Some(ResolutionSource::Env("CLIPROXYAPI_BASE_URL")),
        "NVIDIA_BASE_URL" => Some(ResolutionSource::Env("NVIDIA_BASE_URL")),
        "OPENCODE_BASE_URL" => Some(ResolutionSource::Env("OPENCODE_BASE_URL")),
        PRIMARY_BASE_URL_ENV => Some(ResolutionSource::Env(PRIMARY_BASE_URL_ENV)),
        _ => None,
    }
}

fn fallback_env_name(profile_name: &str) -> &'static str {
    match profile_name {
        "cpa" | "cliproxyapi" => "OPENAI_API_KEY",
        _ => PRIMARY_API_KEY_ENV,
    }
}

fn fallback_profile_api_key(profile_name: &str) -> Option<String> {
    match profile_name {
        "cpa" | "cliproxyapi" => read_non_empty_env("OPENAI_API_KEY"),
        _ => None,
    }
}

fn profile_inline_api_key(runtime_config: &RuntimeConfig, profile_name: &str) -> Option<String> {
    profile_block(runtime_config, profile_name)
        .and_then(|object| object_string(object, &["api_key", "apiKey"]))
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn normalize_provider_model(model: &str) -> String {
    let trimmed = model.trim();
    let Some((prefix, rest)) = trimmed.split_once('/') else {
        return trimmed.to_string();
    };

    if !rest.is_empty()
        && PROVIDER_MODEL_PREFIXES
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(prefix))
    {
        return rest.to_string();
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        CredentialResolver, CredentialSource, ProfileResolver, ProviderLauncher, ResolutionSource,
    };
    use crate::{test_env_lock, ConfigLoader, RuntimeConfig};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("runtime-profile-{nanos}"))
    }

    #[test]
    fn profile_resolver_uses_cli_then_env_then_config_then_default() {
        let _guard = test_env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let home = root.join("home").join(".saicode");
        fs::create_dir_all(&home).expect("home config dir");
        fs::create_dir_all(&cwd).expect("cwd");
        fs::write(home.join("config.toml"), "profile = \"opencode\"\n").expect("write config");
        let runtime_config = ConfigLoader::new(&cwd, &home)
            .load()
            .expect("config should load");

        let resolved = ProfileResolver::resolve(&runtime_config, Some("nvidia"), None)
            .expect("profile should resolve");
        assert_eq!(resolved.profile_name, "nvidia");
        assert_eq!(resolved.profile_source, ResolutionSource::Cli);

        std::env::set_var("SAICODE_PROFILE", "custom");
        let resolved =
            ProfileResolver::resolve(&runtime_config, None, None).expect("profile should resolve");
        assert_eq!(resolved.profile_name, "custom");
        assert_eq!(
            resolved.profile_source,
            ResolutionSource::Env("SAICODE_PROFILE")
        );
        std::env::remove_var("SAICODE_PROFILE");

        let resolved =
            ProfileResolver::resolve(&runtime_config, None, None).expect("profile should resolve");
        assert_eq!(resolved.profile_name, "opencode");
        assert_eq!(
            resolved.profile_source,
            ResolutionSource::Config("config.profile")
        );

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn profile_block_overrides_builtin_defaults() {
        let _guard = test_env_lock();
        std::env::remove_var("SAICODE_BASE_URL");
        std::env::remove_var("CLIPROXYAPI_BASE_URL");
        let root = temp_dir();
        let cwd = root.join("project");
        let home = root.join("home").join(".saicode");
        fs::create_dir_all(&home).expect("home config dir");
        fs::create_dir_all(&cwd).expect("cwd");
        fs::write(
            home.join("config.toml"),
            r#"
profile = "cliproxyapi"
[profiles.cliproxyapi]
base_url = "https://router.example.test/v1"
api_key_env = "ROUTER_TOKEN"
default_model = "gpt-4.1-mini"
request_timeout_ms = 90000
max_retries = 4
"#,
        )
        .expect("write config");
        let runtime_config = ConfigLoader::new(&cwd, &home)
            .load()
            .expect("config should load");

        let resolved =
            ProfileResolver::resolve(&runtime_config, None, None).expect("profile should resolve");
        assert_eq!(
            resolved.base_url.as_deref(),
            Some("https://router.example.test/v1")
        );
        assert_eq!(resolved.profile.api_key_env, "ROUTER_TOKEN");
        assert_eq!(resolved.profile.default_model, "gpt-4.1-mini");
        assert_eq!(resolved.profile.request_timeout_ms, 90_000);
        assert_eq!(resolved.profile.max_retries, 4);

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn credential_resolver_prefers_primary_env_then_profile_env() {
        let _guard = test_env_lock();
        let profile = super::builtin_profile("nvidia").expect("profile");
        std::env::set_var("SAICODE_API_KEY", "primary-key");
        std::env::set_var("SAICODE_BASE_URL", "https://router.example.test");
        let credential = CredentialResolver::resolve(&RuntimeConfig::empty(), &profile);
        assert_eq!(credential.source, CredentialSource::PrimaryEnv);
        assert_eq!(credential.api_key.as_deref(), Some("primary-key"));
        std::env::remove_var("SAICODE_API_KEY");

        std::env::remove_var("SAICODE_API_KEY");
        std::env::set_var(&profile.api_key_env, "profile-key");
        let credential = CredentialResolver::resolve(&RuntimeConfig::empty(), &profile);
        assert_eq!(credential.source, CredentialSource::ProfileEnv);
        assert_eq!(credential.api_key.as_deref(), Some("profile-key"));
        std::env::remove_var(&profile.api_key_env);
        std::env::remove_var("SAICODE_BASE_URL");
    }

    #[test]
    fn provider_launcher_requires_endpoint_and_credentials() {
        let _guard = test_env_lock();
        std::env::remove_var("SAICODE_BASE_URL");
        std::env::remove_var("SAICODE_API_KEY");
        let root = temp_dir();
        let cwd = root.join("project");
        let home = root.join("home").join(".saicode");
        fs::create_dir_all(&home).expect("home config dir");
        fs::create_dir_all(&cwd).expect("cwd");
        fs::write(home.join("config.toml"), "profile = \"custom\"\n").expect("write config");
        let runtime_config = ConfigLoader::new(&cwd, &home)
            .load()
            .expect("config should load");
        let resolved =
            ProfileResolver::resolve(&runtime_config, None, None).expect("profile should resolve");
        let error = ProviderLauncher::prepare(&resolved).expect_err("launch should fail");
        assert!(error.to_string().contains("does not have a base URL"));

        std::env::set_var("SAICODE_BASE_URL", "https://router.example.test");
        std::env::set_var("SAICODE_API_KEY", "test-key");
        let resolved = ProfileResolver::resolve(&runtime_config, None, Some("gpt-4.1-mini"))
            .expect("profile should resolve");
        let launch = ProviderLauncher::prepare(&resolved).expect("launch config");
        assert_eq!(launch.profile_name, "custom");
        assert_eq!(launch.base_url, "https://router.example.test");
        assert_eq!(launch.api_key, "test-key");
        assert_eq!(launch.model, "gpt-4.1-mini");
        std::env::remove_var("SAICODE_BASE_URL");
        std::env::remove_var("SAICODE_API_KEY");

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn provider_launcher_normalizes_provider_prefixed_model_aliases() {
        let _guard = test_env_lock();
        std::env::set_var("SAICODE_BASE_URL", "https://router.example.test");
        std::env::set_var("SAICODE_API_KEY", "test-key");

        let resolved = ProfileResolver::resolve(
            &RuntimeConfig::empty(),
            Some("cliproxyapi"),
            Some("cpa/gpt-5.4"),
        )
        .expect("profile should resolve");
        let launch = ProviderLauncher::prepare(&resolved).expect("launch config");
        assert_eq!(launch.model, "gpt-5.4");

        let resolved = ProfileResolver::resolve(
            &RuntimeConfig::empty(),
            Some("cliproxyapi"),
            Some("cliproxyapi/openai/gpt-oss-120b"),
        )
        .expect("profile should resolve");
        let launch = ProviderLauncher::prepare(&resolved).expect("launch config");
        assert_eq!(launch.model, "openai/gpt-oss-120b");

        let resolved = ProfileResolver::resolve(
            &RuntimeConfig::empty(),
            Some("cliproxyapi"),
            Some("google/gemma-4-31b-it"),
        )
        .expect("profile should resolve");
        let launch = ProviderLauncher::prepare(&resolved).expect("launch config");
        assert_eq!(launch.model, "google/gemma-4-31b-it");

        std::env::remove_var("SAICODE_BASE_URL");
        std::env::remove_var("SAICODE_API_KEY");
    }

    #[test]
    fn available_profile_names_include_config_extensions() {
        let _guard = test_env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let home = root.join("home").join(".saicode");
        fs::create_dir_all(&home).expect("home config dir");
        fs::create_dir_all(&cwd).expect("cwd");
        fs::write(
            home.join("config.toml"),
            r#"
[profiles.router]
base_url = "https://router.example.test/v1"
"#,
        )
        .expect("write config");
        let runtime_config = ConfigLoader::new(&cwd, &home)
            .load()
            .expect("config should load");

        let names = ProfileResolver::available_profile_names(&runtime_config);
        assert!(names.contains(&"cpa".to_string()));
        assert!(names.contains(&"cliproxyapi".to_string()));
        assert!(names.contains(&"custom".to_string()));
        assert!(names.contains(&"router".to_string()));

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn profile_resolver_reads_saicode_config_json_providers_block() {
        let _guard = test_env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let home = root.join("home").join(".saicode");
        fs::create_dir_all(&home).expect("home config dir");
        fs::create_dir_all(&cwd).expect("cwd");
        fs::write(
            home.join("config.json"),
            r#"{
              "profile": "cliproxyapi",
              "providers": {
                "cliproxyapi": {
                  "baseUrl": "http://127.0.0.1:8317/v1",
                  "apiKey": "from-file"
                }
              }
            }"#,
        )
        .expect("write config json");

        let runtime_config = ConfigLoader::new(&cwd, &home)
            .load()
            .expect("config should load");
        let resolved =
            ProfileResolver::resolve(&runtime_config, None, None).expect("profile should resolve");

        assert_eq!(resolved.profile_name, "cliproxyapi");
        assert_eq!(
            resolved.base_url.as_deref(),
            Some("http://127.0.0.1:8317/v1")
        );
        assert_eq!(resolved.credential.source, CredentialSource::ConfigValue);
        assert_eq!(resolved.credential.api_key.as_deref(), Some("from-file"));

        fs::remove_dir_all(root).expect("cleanup");
    }
}
