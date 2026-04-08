use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const NO_CONTENT_MESSAGE: &str = "(no content)";
const SYNTHETIC_MODEL: &str = "<synthetic>";
const DEFAULT_MODEL_ID: &str = "cpa/qwen/qwen3.5-122b-a10b";
const DEFAULT_TIMEOUT_MS: u64 = 600_000;

const RECOVERY_HELP_TEXT: &str = r#"Usage: saicode [options] [prompt]

Local recovery mode for saicode.

Options:
  -h, --help                    Show help
  -v, --version                 Show version
  (no args)                     Start local interactive mode
  -p, --print                   Send a single prompt and print the result
  --model <model>               Override model
  --system-prompt <text>        Override system prompt
  --system-prompt-file <file>   Read system prompt from file
  --append-system-prompt <text> Append to the system prompt
  --output-format <format>      text (default) or json

Environment:
  SAICODE_MODEL
  SAICODE_PROVIDER
  SAICODE_CONFIG_DIR
  SAICODE_PROVIDER=cpa
  CPA_API_KEY / CPA_BASE_URL
  CLIPROXYAPI_API_KEY / CLIPROXYAPI_BASE_URL
  API_TIMEOUT_MS
"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryCommand {
    Help,
    Version,
    Run,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireApi {
    OpenAIResponses,
    OpenAIChatCompletions,
}

#[derive(Clone, Debug)]
pub struct ParsedRecoveryArgs {
    pub command: RecoveryCommand,
    pub print: bool,
    model: Option<String>,
    system_prompt: Option<String>,
    append_system_prompt: Option<String>,
    output_format: OutputFormat,
    prompt: String,
}

#[derive(Clone, Debug)]
pub struct ResolvedModel {
    #[allow(dead_code)]
    pub alias: String,
    pub provider: String,
    pub model: String,
    pub max_output_tokens: u32,
}

#[derive(Clone, Debug)]
pub struct ProviderConfig {
    pub id: String,
    pub api: WireApi,
    pub base_url: String,
    pub api_key: Option<String>,
    pub headers: HashMap<String, String>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Usage {
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
}

#[derive(Clone, Debug)]
struct RecoveryResponse {
    text: String,
    usage: Usage,
}

#[derive(Clone, Debug)]
pub(crate) struct StreamedToolCall {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) input: Value,
}

#[derive(Clone, Debug)]
pub(crate) struct StreamedChatCompletion {
    pub(crate) text: String,
    pub(crate) tool_calls: Vec<StreamedToolCall>,
    pub(crate) usage: Usage,
}

#[derive(Debug, Default, Deserialize)]
struct ChatCompletionChunk {
    #[serde(default)]
    choices: Vec<ChunkChoice>,
    #[serde(default)]
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    #[serde(default)]
    delta: ChunkDelta,
}

#[derive(Debug, Default, Deserialize)]
struct ChunkDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    tool_calls: Vec<DeltaToolCall>,
}

#[derive(Debug, Default, Deserialize)]
struct DeltaToolCall {
    #[serde(default)]
    index: u32,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: DeltaFunction,
}

#[derive(Debug, Default, Deserialize)]
struct DeltaFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ChatUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
}

#[derive(Debug, Default)]
struct StreamedToolCallState {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

#[derive(Clone, Copy)]
struct ModelEntry {
    id: &'static str,
    provider: &'static str,
    model: &'static str,
    max_output_tokens: u32,
    aliases: &'static [&'static str],
}

const MODEL_CATALOG: &[ModelEntry] = &[
    ModelEntry {
        id: "cpa/qwen/qwen3.5-122b-a10b",
        provider: "cpa",
        model: "qwen/qwen3.5-122b-a10b",
        max_output_tokens: 24576,
        aliases: &[
            "qwen-fast",
            "qwen-122b",
            "cliproxyapi/qwen/qwen3.5-122b-a10b",
            "nvidia/qwen/qwen3.5-122b-a10b",
            "cliproxy-qwen-fast",
        ],
    },
    ModelEntry {
        id: "cpa/qwen/qwen3.5-397b-a17b",
        provider: "cpa",
        model: "qwen/qwen3.5-397b-a17b",
        max_output_tokens: 24576,
        aliases: &[
            "qwen-max",
            "qwen-397b",
            "qwen397",
            "best",
            "default",
            "cliproxyapi/qwen/qwen3.5-397b-a17b",
            "nvidia/qwen/qwen3.5-397b-a17b",
            "cliproxy-qwen-max",
        ],
    },
    ModelEntry {
        id: "cpa/qwen3-coder-plus",
        provider: "cpa",
        model: "qwen3-coder-plus",
        max_output_tokens: 32768,
        aliases: &[
            "qwen-coder-plus",
            "qwen-coder",
            "cliproxyapi/qwen3-coder-plus",
        ],
    },
    ModelEntry {
        id: "cpa/vision-model",
        provider: "cpa",
        model: "vision-model",
        max_output_tokens: 32768,
        aliases: &[
            "qwen-vision",
            "qwen3-vision",
            "vision",
            "cliproxyapi/vision-model",
        ],
    },
    ModelEntry {
        id: "cpa/nvidia/nemotron-3-super-120b-a12b",
        provider: "cpa",
        model: "nvidia/nemotron-3-super-120b-a12b",
        max_output_tokens: 32768,
        aliases: &[
            "nemotron",
            "cliproxy-nemotron",
            "cliproxyapi/nvidia/nemotron-3-super-120b-a12b",
            "nvidia/nvidia/nemotron-3-super-120b-a12b",
        ],
    },
    ModelEntry {
        id: "cpa/openai/gpt-oss-120b",
        provider: "cpa",
        model: "openai/gpt-oss-120b",
        max_output_tokens: 32768,
        aliases: &[
            "gpt-oss",
            "cliproxy-gpt-oss",
            "cliproxyapi/openai/gpt-oss-120b",
            "nvidia/openai/gpt-oss-120b",
        ],
    },
    ModelEntry {
        id: "cpa/google/gemma-4-31b-it",
        provider: "cpa",
        model: "google/gemma-4-31b-it",
        max_output_tokens: 32768,
        aliases: &[
            "gemma4",
            "gemma-4",
            "gemma-31b",
            "cliproxyapi/google/gemma-4-31b-it",
            "nvidia/google/gemma-4-31b-it",
        ],
    },
    ModelEntry {
        id: "cpa/gpt-5.4",
        provider: "cpa",
        model: "gpt-5.4",
        max_output_tokens: 32768,
        aliases: &["codex", "gpt-5.4", "cliproxyapi/gpt-5.4"],
    },
    ModelEntry {
        id: "cpa/gpt-5.4-mini",
        provider: "cpa",
        model: "gpt-5.4-mini",
        max_output_tokens: 32768,
        aliases: &["codex-mini", "gpt-5.4-mini", "cliproxyapi/gpt-5.4-mini"],
    },
    ModelEntry {
        id: "cpa/opencode/qwen3.6-plus-free",
        provider: "cpa",
        model: "qwen3.6-plus-free",
        max_output_tokens: 64000,
        aliases: &[
            "opencode-qwen-free",
            "qwen3.6-free",
            "cliproxyapi/opencode/qwen3.6-plus-free",
        ],
    },
    ModelEntry {
        id: "cpa/opencode/mimo-v2-pro-free",
        provider: "cpa",
        model: "mimo-v2-pro-free",
        max_output_tokens: 64000,
        aliases: &[
            "opencode-mimo-pro-free",
            "mimo-pro-free",
            "cliproxyapi/opencode/mimo-v2-pro-free",
        ],
    },
    ModelEntry {
        id: "cpa/opencode/mimo-v2-omni-free",
        provider: "cpa",
        model: "mimo-v2-omni-free",
        max_output_tokens: 64000,
        aliases: &[
            "opencode-mimo-omni-free",
            "mimo-omni-free",
            "cliproxyapi/opencode/mimo-v2-omni-free",
        ],
    },
];

#[derive(Default, Deserialize)]
struct RuntimeConfigFile {
    providers: Option<HashMap<String, RuntimeProviderConfig>>,
}

#[derive(Clone, Default, Deserialize)]
struct RuntimeProviderConfig {
    api: Option<String>,
    #[serde(rename = "baseUrl")]
    base_url: Option<String>,
    #[serde(rename = "apiKey")]
    api_key: Option<String>,
    headers: Option<HashMap<String, String>>,
}

pub fn should_handle_natively(args: &[String]) -> bool {
    if is_env_truthy(env::var("SAICODE_DISABLE_NATIVE_RECOVERY").ok().as_deref()) {
        return false;
    }

    matches!(
        parse_recovery_args(args),
        Ok(parsed) if matches!(parsed.command, RecoveryCommand::Help | RecoveryCommand::Version)
            || parsed.print
    )
}

pub fn run_native_recovery(args: &[String], version: &str) -> Result<(), String> {
    let parsed = parse_recovery_args(args)?;

    match parsed.command {
        RecoveryCommand::Help => {
            print!("{RECOVERY_HELP_TEXT}");
            return Ok(());
        }
        RecoveryCommand::Version => {
            println!("{version} (saicode recovery)");
            return Ok(());
        }
        RecoveryCommand::Run => {}
    }

    if !parsed.print {
        return Err("Native recovery only handles --print requests".to_string());
    }

    let prompt = if parsed.prompt.trim().is_empty() {
        read_prompt_from_stdin()?
    } else {
        parsed.prompt.clone()
    };
    if prompt.trim().is_empty() {
        return Err("Error: prompt is required".to_string());
    }

    let resolved_model = resolve_model(parsed.model.as_deref());
    let provider = get_provider_config(&resolved_model)?;
    let system_prompt = join_system_prompt(
        parsed.system_prompt.as_deref(),
        parsed.append_system_prompt.as_deref(),
    );
    let response = query_saicode(
        &prompt,
        system_prompt.as_deref(),
        &resolved_model,
        &provider,
        parsed.output_format == OutputFormat::Text,
    )?;

    match parsed.output_format {
        OutputFormat::Text => {
            if response.text.is_empty() {
                println!();
            }
        }
        OutputFormat::Json => {
            let payload = build_json_output(&response, &resolved_model)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&payload)
                    .map_err(|error| format!("Failed to render JSON output: {error}"))?
            );
        }
    }

    Ok(())
}

fn parse_recovery_args(args: &[String]) -> Result<ParsedRecoveryArgs, String> {
    let mut parsed = ParsedRecoveryArgs {
        command: RecoveryCommand::Run,
        print: false,
        model: None,
        system_prompt: None,
        append_system_prompt: None,
        output_format: OutputFormat::Text,
        prompt: String::new(),
    };
    let mut positional = Vec::new();

    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--" => {
                positional.extend_from_slice(&args[index + 1..]);
                break;
            }
            "-h" | "--help" => {
                parsed.command = RecoveryCommand::Help;
                return Ok(parsed);
            }
            "-v" | "-V" | "--version" => {
                parsed.command = RecoveryCommand::Version;
                return Ok(parsed);
            }
            "-p" | "--print" => {
                parsed.print = true;
            }
            "--bare" | "--dangerously-skip-permissions" => {}
            "--model" => {
                parsed.model = Some(read_flag_value(args, index, "--model")?);
                index += 1;
            }
            "--system-prompt" => {
                parsed.system_prompt = Some(read_flag_value(args, index, "--system-prompt")?);
                index += 1;
            }
            "--system-prompt-file" => {
                let path = read_flag_value(args, index, "--system-prompt-file")?;
                parsed.system_prompt = Some(
                    fs::read_to_string(&path)
                        .map_err(|error| format!("Failed to read {}: {error}", path))?,
                );
                index += 1;
            }
            "--append-system-prompt" => {
                parsed.append_system_prompt =
                    Some(read_flag_value(args, index, "--append-system-prompt")?);
                index += 1;
            }
            "--output-format" => {
                let value = read_flag_value(args, index, "--output-format")?;
                parsed.output_format = match value.as_str() {
                    "json" => OutputFormat::Json,
                    _ => OutputFormat::Text,
                };
                index += 1;
            }
            _ if arg.starts_with('-') => {
                return Err(format!("Unsupported flag for native recovery path: {arg}"));
            }
            _ => positional.push(args[index].clone()),
        }

        index += 1;
    }

    parsed.prompt = positional.join(" ").trim().to_string();
    Ok(parsed)
}

fn read_flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .filter(|value| !value.starts_with('-'))
        .cloned()
        .ok_or_else(|| format!("Missing value for {flag}"))
}

pub(crate) fn read_prompt_from_stdin() -> Result<String, String> {
    if io::stdin().is_terminal() {
        return Ok(String::new());
    }

    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|error| format!("Failed to read stdin: {error}"))?;
    Ok(buffer.trim().to_string())
}

pub(crate) fn join_system_prompt(
    system_prompt: Option<&str>,
    append_system_prompt: Option<&str>,
) -> Option<String> {
    match (system_prompt, append_system_prompt) {
        (Some(base), Some(extra)) => Some(format!("{base}\n\n{extra}")),
        (Some(base), None) => Some(base.to_string()),
        (None, Some(extra)) => Some(extra.to_string()),
        (None, None) => None,
    }
}

fn get_model_entry(model_input: &str) -> Option<&'static ModelEntry> {
    let normalized = model_input.trim().to_ascii_lowercase();
    MODEL_CATALOG.iter().find(|entry| {
        entry.id.eq_ignore_ascii_case(&normalized)
            || entry
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(&normalized))
    })
}

fn get_default_model_id() -> String {
    env::var("SAICODE_DEFAULT_MODEL").unwrap_or_else(|_| DEFAULT_MODEL_ID.to_string())
}

fn resolve_model_id(model_input: Option<&str>) -> String {
    let candidate = model_input
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(get_default_model_id);

    get_model_entry(&candidate)
        .map(|entry| entry.id.to_string())
        .unwrap_or(candidate)
}

fn resolve_recovery_model_override(model: Option<&str>) -> Option<String> {
    model
        .map(ToOwned::to_owned)
        .or_else(|| env::var("SAICODE_MODEL").ok())
        .or_else(|| env::var("SAICODE_DEFAULT_MODEL").ok())
        .or_else(|| env::var("SAICODE_DEFAULT_SONNET_MODEL").ok())
        .or_else(|| Some(DEFAULT_MODEL_ID.to_string()))
}

pub fn resolve_model(model_input: Option<&str>) -> ResolvedModel {
    let resolved_id = resolve_model_id(resolve_recovery_model_override(model_input).as_deref());
    if let Some(entry) = get_model_entry(&resolved_id) {
        return ResolvedModel {
            alias: entry.id.to_string(),
            provider: entry.provider.to_string(),
            model: entry.model.to_string(),
            max_output_tokens: entry.max_output_tokens,
        };
    }

    let inferred_provider = env::var("SAICODE_PROVIDER")
        .ok()
        .or_else(|| env::var("SAICODE_DEFAULT_PROVIDER").ok())
        .unwrap_or_else(|| {
            if resolved_id.starts_with("cpa/") {
                "cpa".to_string()
            } else if resolved_id.starts_with("cliproxyapi/") {
                "cliproxyapi".to_string()
            } else if resolved_id.starts_with("nvidia/") {
                "nvidia".to_string()
            } else {
                "cpa".to_string()
            }
        });

    if resolved_id.contains('/') {
        let mut parts = resolved_id.split('/');
        let _ = parts.next();
        let model = parts.collect::<Vec<_>>().join("/");
        return ResolvedModel {
            alias: resolved_id.clone(),
            provider: inferred_provider,
            model: if model.is_empty() { resolved_id } else { model },
            max_output_tokens: 32768,
        };
    }

    ResolvedModel {
        alias: resolved_id.clone(),
        provider: inferred_provider,
        model: resolved_id,
        max_output_tokens: 32768,
    }
}

fn get_config_home_dir() -> PathBuf {
    if let Ok(path) = env::var("SAICODE_CONFIG_DIR") {
        return PathBuf::from(path);
    }
    if let Ok(path) = env::var("CLAUDE_CONFIG_DIR") {
        return PathBuf::from(path);
    }
    if let Ok(home) = env::var("HOME") {
        return Path::new(&home).join(".saicode");
    }
    if let Ok(home) = env::var("USERPROFILE") {
        return PathBuf::from(home).join(".saicode");
    }
    PathBuf::from(".saicode")
}

fn get_runtime_config_file() -> RuntimeConfigFile {
    let path = get_config_home_dir().join("config.json");
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str::<RuntimeConfigFile>(&contents).unwrap_or_default(),
        Err(_) => RuntimeConfigFile::default(),
    }
}

fn parse_wire_api(value: Option<&str>, default: WireApi) -> WireApi {
    match value {
        Some("openai-chat-completions") => WireApi::OpenAIChatCompletions,
        Some("openai-responses") => WireApi::OpenAIResponses,
        _ => default,
    }
}

pub fn get_provider_config(resolved_model: &ResolvedModel) -> Result<ProviderConfig, String> {
    let runtime_config = get_runtime_config_file();
    let provider_keys: Vec<&str> = match resolved_model.provider.as_str() {
        "cpa" => vec!["cpa", "cliproxyapi"],
        "cliproxyapi" => vec!["cliproxyapi", "cpa"],
        other => vec![other],
    };
    let file_provider = provider_keys.iter().find_map(|key| {
        runtime_config
            .providers
            .as_ref()
            .and_then(|providers| providers.get(*key))
            .cloned()
    });

    match resolved_model.provider.as_str() {
        "cpa" | "cliproxyapi" => Ok(ProviderConfig {
            id: resolved_model.provider.clone(),
            api: parse_wire_api(
                env::var("CPA_API")
                    .ok()
                    .or_else(|| env::var("CLIPROXYAPI_API").ok())
                    .or_else(|| {
                        file_provider
                            .as_ref()
                            .and_then(|provider| provider.api.clone())
                    })
                    .as_deref(),
                WireApi::OpenAIChatCompletions,
            ),
            base_url: env::var("CPA_BASE_URL")
                .ok()
                .or_else(|| env::var("CLIPROXYAPI_BASE_URL").ok())
                .or_else(|| {
                    file_provider
                        .as_ref()
                        .and_then(|provider| provider.base_url.clone())
                })
                .unwrap_or_else(|| "http://127.0.0.1:8317/v1".to_string()),
            api_key: env::var("CPA_API_KEY")
                .ok()
                .or_else(|| env::var("CLIPROXYAPI_API_KEY").ok())
                .or_else(|| {
                    file_provider
                        .as_ref()
                        .and_then(|provider| provider.api_key.clone())
                })
                .or_else(|| env::var("OPENAI_API_KEY").ok()),
            headers: file_provider
                .as_ref()
                .and_then(|provider| provider.headers.clone())
                .unwrap_or_default(),
        }),
        _ => Ok(ProviderConfig {
            id: "nvidia".to_string(),
            api: parse_wire_api(
                env::var("NVIDIA_API")
                    .ok()
                    .or_else(|| {
                        file_provider
                            .as_ref()
                            .and_then(|provider| provider.api.clone())
                    })
                    .as_deref(),
                WireApi::OpenAIChatCompletions,
            ),
            base_url: env::var("NVIDIA_BASE_URL")
                .ok()
                .or_else(|| {
                    file_provider
                        .as_ref()
                        .and_then(|provider| provider.base_url.clone())
                })
                .unwrap_or_else(|| "https://integrate.api.nvidia.com/v1".to_string()),
            api_key: env::var("NVIDIA_API_KEY").ok().or_else(|| {
                file_provider
                    .as_ref()
                    .and_then(|provider| provider.api_key.clone())
            }),
            headers: file_provider
                .as_ref()
                .and_then(|provider| provider.headers.clone())
                .unwrap_or_default(),
        }),
    }
}

fn get_missing_api_key_message(provider_id: &str) -> String {
    if provider_id == "cpa" || provider_id == "cliproxyapi" {
        return format!(
            "{provider_id} API key is missing. Set CPA_API_KEY or CLIPROXYAPI_API_KEY (OPENAI_API_KEY also works), or configure ~/.saicode/config.json providers.cpa/cliproxyapi.apiKey"
        );
    }

    format!(
        "{provider_id} API key is missing. Set {}_API_KEY or configure ~/.saicode/config.json providers.{provider_id}.apiKey",
        provider_id.to_ascii_uppercase()
    )
}

fn request_timeout_ms() -> u64 {
    env::var("API_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_TIMEOUT_MS)
}

fn build_headers(provider: &ProviderConfig) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    if let Some(api_key) = provider.api_key.as_deref() {
        headers.insert("Authorization".to_string(), format!("Bearer {api_key}"));
    }

    for (key, value) in &provider.headers {
        headers.insert(key.clone(), value.clone());
    }

    Ok(headers)
}

fn build_provider_curl_command(
    provider: &ProviderConfig,
    endpoint: &str,
    body_text: String,
) -> Result<Command, String> {
    let headers = build_headers(provider)?;
    let mut command = Command::new("curl");
    command.arg("-sS");
    command.arg("-X").arg("POST");
    command.arg(endpoint);
    command
        .arg("--max-time")
        .arg(format!("{:.3}", request_timeout_ms() as f64 / 1000.0));
    for (key, value) in &headers {
        command.arg("-H").arg(format!("{key}: {value}"));
    }
    command.arg("--data-binary").arg(body_text);
    Ok(command)
}

pub(crate) fn execute_provider_json_request(
    provider: &ProviderConfig,
    body: &Value,
) -> Result<Value, String> {
    if provider.api_key.is_none() && provider.id != "cliproxyapi" {
        return Err(get_missing_api_key_message(&provider.id));
    }

    let endpoint = match provider.api {
        WireApi::OpenAIResponses => format!("{}/responses", provider.base_url),
        WireApi::OpenAIChatCompletions => format!("{}/chat/completions", provider.base_url),
    };

    let mut command = build_provider_curl_command(provider, &endpoint, body.to_string())?;
    command
        .arg("--write-out")
        .arg("\n__SAICODE_STATUS__:%{http_code}");

    let output = command
        .output()
        .map_err(|error| format!("Failed to execute curl: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "saicode request failed".to_string()
        } else {
            format!("saicode request failed: {stderr}")
        });
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("Provider response was not valid UTF-8: {error}"))?;
    let marker = "\n__SAICODE_STATUS__:";
    let split_index = stdout
        .rfind(marker)
        .ok_or_else(|| "Failed to parse curl status marker".to_string())?;
    let response_text = stdout[..split_index].to_string();
    let status_code = stdout[split_index + marker.len()..]
        .trim()
        .parse::<u16>()
        .map_err(|error| format!("Invalid curl status code: {error}"))?;

    if !(200..300).contains(&status_code) {
        return Err(response_text);
    }

    serde_json::from_str(&response_text)
        .map_err(|error| format!("Failed to parse provider response: {error}"))
}

pub(crate) fn execute_provider_chat_completions_stream(
    provider: &ProviderConfig,
    body: &Value,
    stream_to_stdout: bool,
) -> Result<StreamedChatCompletion, String> {
    if provider.api != WireApi::OpenAIChatCompletions {
        return Err("Streaming helper only supports chat completions".to_string());
    }
    if provider.api_key.is_none() && provider.id != "cliproxyapi" {
        return Err(get_missing_api_key_message(&provider.id));
    }

    let endpoint = format!("{}/chat/completions", provider.base_url);
    let mut body = body.clone();
    if let Some(object) = body.as_object_mut() {
        object.insert("stream".to_string(), Value::Bool(true));
    }

    let mut command = build_provider_curl_command(provider, &endpoint, body.to_string())?;
    command.arg("-N");
    command
        .arg("--write-out")
        .arg("\n__SAICODE_STATUS__:%{http_code}");
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|error| format!("Failed to execute curl: {error}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture curl stdout".to_string())?;
    let mut raw_stdout = Vec::new();
    let mut read_buffer = [0_u8; 4096];
    let mut stream_state = ChatCompletionStreamState::default();

    loop {
        let bytes_read = stdout
            .read(&mut read_buffer)
            .map_err(|error| format!("Failed to read curl stdout: {error}"))?;
        if bytes_read == 0 {
            break;
        }
        let chunk = &read_buffer[..bytes_read];
        raw_stdout.extend_from_slice(chunk);
        stream_state.ingest_bytes(chunk, stream_to_stdout)?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("Failed to wait for curl: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "saicode request failed".to_string()
        } else {
            format!("saicode request failed: {stderr}")
        });
    }

    let stdout_text = String::from_utf8(raw_stdout)
        .map_err(|error| format!("Provider response was not valid UTF-8: {error}"))?;
    let marker = "\n__SAICODE_STATUS__:";
    let split_index = stdout_text
        .rfind(marker)
        .ok_or_else(|| "Failed to parse curl status marker".to_string())?;
    let status_code = stdout_text[split_index + marker.len()..]
        .trim()
        .parse::<u16>()
        .map_err(|error| format!("Invalid curl status code: {error}"))?;
    if !(200..300).contains(&status_code) {
        let response_text = stdout_text[..split_index].trim().to_string();
        return Err(response_text);
    }

    if stream_to_stdout && !stream_state.text.is_empty() {
        io::stdout()
            .write_all(b"\n")
            .and_then(|_| io::stdout().flush())
            .map_err(|error| format!("Failed to flush stdout: {error}"))?;
    }

    Ok(stream_state.finish())
}

#[derive(Default)]
struct ChatCompletionStreamState {
    frame_buffer: Vec<u8>,
    text: String,
    usage: Usage,
    tool_calls: BTreeMap<u32, StreamedToolCallState>,
}

impl ChatCompletionStreamState {
    fn ingest_bytes(&mut self, bytes: &[u8], stream_to_stdout: bool) -> Result<(), String> {
        self.frame_buffer.extend_from_slice(bytes);
        while let Some(frame) = next_sse_frame(&mut self.frame_buffer) {
            if let Some(chunk) = parse_sse_frame(&frame)? {
                self.ingest_chunk(chunk, stream_to_stdout)?;
            }
        }
        Ok(())
    }

    fn ingest_chunk(
        &mut self,
        chunk: ChatCompletionChunk,
        stream_to_stdout: bool,
    ) -> Result<(), String> {
        if let Some(usage) = chunk.usage {
            self.usage = Usage {
                input_tokens: usage.prompt_tokens,
                output_tokens: usage.completion_tokens,
            };
        }

        for choice in chunk.choices {
            if let Some(content) = choice.delta.content.filter(|value| !value.is_empty()) {
                self.text.push_str(&content);
                if stream_to_stdout {
                    io::stdout()
                        .write_all(content.as_bytes())
                        .and_then(|_| io::stdout().flush())
                        .map_err(|error| format!("Failed to stream stdout: {error}"))?;
                }
            }

            for tool_call in choice.delta.tool_calls {
                let state = self.tool_calls.entry(tool_call.index).or_default();
                if let Some(id) = tool_call.id {
                    state.id = Some(id);
                }
                if let Some(name) = tool_call.function.name {
                    state.name = Some(name);
                }
                if let Some(arguments) = tool_call.function.arguments {
                    state.arguments.push_str(&arguments);
                }
            }
        }

        Ok(())
    }

    fn finish(self) -> StreamedChatCompletion {
        let tool_calls = self
            .tool_calls
            .into_iter()
            .map(|(index, state)| StreamedToolCall {
                id: state.id.unwrap_or_else(|| format!("tool_call_{index}")),
                name: state.name.unwrap_or_default(),
                input: parse_tool_arguments_text(&state.arguments),
            })
            .collect();

        StreamedChatCompletion {
            text: self.text,
            tool_calls,
            usage: self.usage,
        }
    }
}

fn next_sse_frame(buffer: &mut Vec<u8>) -> Option<String> {
    let separator = buffer
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|position| (position, 2))
        .or_else(|| {
            buffer
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|position| (position, 4))
        })?;

    let (position, separator_len) = separator;
    let frame = buffer.drain(..position + separator_len).collect::<Vec<_>>();
    let frame_len = frame.len().saturating_sub(separator_len);
    Some(String::from_utf8_lossy(&frame[..frame_len]).into_owned())
}

fn parse_sse_frame(frame: &str) -> Result<Option<ChatCompletionChunk>, String> {
    let trimmed = frame.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let mut data_lines = Vec::new();
    for line in trimmed.lines() {
        if line.starts_with(':') {
            continue;
        }
        if let Some(data) = line.strip_prefix("data:") {
            data_lines.push(data.trim_start());
        }
    }
    if data_lines.is_empty() {
        return Ok(None);
    }

    let payload = data_lines.join("\n");
    if payload == "[DONE]" {
        return Ok(None);
    }

    serde_json::from_str(&payload)
        .map(Some)
        .map_err(|error| format!("Failed to parse SSE frame: {error}"))
}

fn parse_tool_arguments_text(arguments: &str) -> Value {
    if arguments.trim().is_empty() {
        return json!({});
    }
    serde_json::from_str::<Value>(arguments).unwrap_or_else(|_| json!({ "raw": arguments }))
}

fn query_saicode(
    prompt: &str,
    system_prompt: Option<&str>,
    resolved_model: &ResolvedModel,
    provider: &ProviderConfig,
    stream_to_stdout: bool,
) -> Result<RecoveryResponse, String> {
    let body = match provider.api {
        WireApi::OpenAIResponses => {
            let mut body = json!({
                "model": resolved_model.model,
                "input": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "input_text",
                                "text": prompt,
                            }
                        ],
                    }
                ],
                "parallel_tool_calls": false,
                "max_output_tokens": resolved_model.max_output_tokens,
            });
            if let Some(system) = system_prompt {
                body["instructions"] = Value::String(system.to_string());
            }
            body
        }
        WireApi::OpenAIChatCompletions => {
            let mut messages = Vec::new();
            if let Some(system) = system_prompt {
                messages.push(json!({ "role": "system", "content": system }));
            }
            messages.push(json!({ "role": "user", "content": prompt }));
            json!({
                "model": resolved_model.model,
                "messages": messages,
                "max_tokens": resolved_model.max_output_tokens,
            })
        }
    };

    let (text, usage) = match provider.api {
        WireApi::OpenAIChatCompletions => {
            let streamed =
                execute_provider_chat_completions_stream(provider, &body, stream_to_stdout)?;
            (streamed.text, streamed.usage)
        }
        WireApi::OpenAIResponses => {
            let json = execute_provider_json_request(provider, &body)?;
            (
                extract_response_text(&json, provider.api),
                extract_usage(&json, provider.api),
            )
        }
    };

    Ok(RecoveryResponse { text, usage })
}

fn extract_response_text(json: &Value, api: WireApi) -> String {
    match api {
        WireApi::OpenAIResponses => {
            if let Some(text) = json.get("output_text").and_then(Value::as_str) {
                if !text.is_empty() {
                    return text.to_string();
                }
            }

            let mut parts = Vec::new();
            if let Some(items) = json.get("output").and_then(Value::as_array) {
                for item in items {
                    if item.get("type").and_then(Value::as_str) != Some("message") {
                        continue;
                    }
                    if let Some(contents) = item.get("content").and_then(Value::as_array) {
                        for content in contents {
                            if content.get("type").and_then(Value::as_str) == Some("output_text") {
                                if let Some(text) = content.get("text").and_then(Value::as_str) {
                                    parts.push(text.to_string());
                                }
                            }
                        }
                    }
                }
            }
            parts.join("\n\n")
        }
        WireApi::OpenAIChatCompletions => {
            let content = &json["choices"][0]["message"]["content"];
            if let Some(text) = content.as_str() {
                return text.to_string();
            }
            if let Some(items) = content.as_array() {
                let parts: Vec<String> = items
                    .iter()
                    .filter_map(|item| {
                        item.get("text")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                            .or_else(|| {
                                item.get("text")
                                    .and_then(|nested| nested.get("value"))
                                    .and_then(Value::as_str)
                                    .map(ToOwned::to_owned)
                            })
                    })
                    .collect();
                return parts.join("\n\n");
            }
            String::new()
        }
    }
}

pub(crate) fn extract_usage(json: &Value, api: WireApi) -> Usage {
    let usage = &json["usage"];
    match api {
        WireApi::OpenAIResponses => Usage {
            input_tokens: usage
                .get("input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            output_tokens: usage
                .get("output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        },
        WireApi::OpenAIChatCompletions => Usage {
            input_tokens: usage
                .get("prompt_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            output_tokens: usage
                .get("completion_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        },
    }
}

fn build_json_output(
    response: &RecoveryResponse,
    _resolved_model: &ResolvedModel,
) -> Result<Value, String> {
    build_json_output_from_text(&response.text, response.usage)
}

pub(crate) fn build_json_output_from_text(text: &str, usage: Usage) -> Result<Value, String> {
    let timestamp = synthetic_timestamp()?;
    let message_uuid = synthetic_uuid_like();
    let turn_uuid = synthetic_uuid_like();
    Ok(json!({
        "type": "assistant",
        "uuid": turn_uuid,
        "timestamp": timestamp,
        "message": {
            "id": message_uuid,
            "container": Value::Null,
            "model": SYNTHETIC_MODEL,
            "role": "assistant",
            "stop_reason": "stop_sequence",
            "stop_sequence": "",
            "type": "message",
            "usage": {
                "input_tokens": usage.input_tokens,
                "output_tokens": usage.output_tokens,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
                "server_tool_use": {
                    "web_search_requests": 0,
                    "web_fetch_requests": 0
                },
                "service_tier": Value::Null,
                "cache_creation": {
                    "ephemeral_1h_input_tokens": 0,
                    "ephemeral_5m_input_tokens": 0
                },
                "inference_geo": Value::Null,
                "iterations": Value::Null,
                "speed": Value::Null
            },
            "content": [
                {
                    "type": "text",
                    "text": if text.is_empty() { NO_CONTENT_MESSAGE } else { text }
                }
            ],
            "context_management": Value::Null
        },
        "isApiErrorMessage": false
    }))
}

fn synthetic_timestamp() -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Clock error: {error}"))?;
    Ok(format!("{}", now.as_secs()))
}

fn synthetic_uuid_like() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("native-{now:x}")
}

fn is_env_truthy(value: Option<&str>) -> bool {
    matches!(
        value.map(|item| item.trim().to_ascii_lowercase()),
        Some(ref item) if matches!(item.as_str(), "1" | "true" | "yes" | "on")
    )
}

type HeaderMap = HashMap<String, String>;

#[cfg(test)]
mod tests {
    use super::{
        get_provider_config, parse_recovery_args, resolve_model, resolve_model_id,
        should_handle_natively, OutputFormat, RecoveryCommand,
    };
    use std::env;
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn reset_env(keys: &[&str]) {
        for key in keys {
            env::remove_var(key);
        }
    }

    #[test]
    fn parses_native_recovery_flags() {
        let parsed = parse_recovery_args(&args(&[
            "-p",
            "--model",
            "cpa/gpt-5.4",
            "--system-prompt",
            "system",
            "--append-system-prompt",
            "append",
            "--output-format",
            "json",
            "hello",
        ]))
        .expect("parse should succeed");

        assert_eq!(parsed.command, RecoveryCommand::Run);
        assert!(parsed.print);
        assert_eq!(parsed.model.as_deref(), Some("cpa/gpt-5.4"));
        assert_eq!(parsed.system_prompt.as_deref(), Some("system"));
        assert_eq!(parsed.append_system_prompt.as_deref(), Some("append"));
        assert_eq!(parsed.output_format, OutputFormat::Json);
        assert_eq!(parsed.prompt, "hello");
    }

    #[test]
    fn native_recovery_rejects_unknown_flags() {
        assert!(parse_recovery_args(&args(&["-p", "--resume", "abc"])).is_err());
        assert!(!should_handle_natively(&args(&["-p", "--resume", "abc"])));
    }

    #[test]
    fn native_recovery_accepts_double_dash_prompt_separator() {
        let parsed = parse_recovery_args(&args(&["-p", "--", "hello", "world"]))
            .expect("parse should succeed");
        assert!(parsed.print);
        assert_eq!(parsed.prompt, "hello world");
    }

    #[test]
    fn keeps_default_catalog_on_qwen_122b_baseline() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        reset_env(&[
            "SAICODE_MODEL",
            "SAICODE_DEFAULT_MODEL",
            "SAICODE_DEFAULT_SONNET_MODEL",
            "SAICODE_PROVIDER",
            "SAICODE_DEFAULT_PROVIDER",
        ]);

        assert_eq!(resolve_model_id(None), "cpa/qwen/qwen3.5-122b-a10b");
        let resolved = resolve_model(None);
        assert_eq!(resolved.alias, "cpa/qwen/qwen3.5-122b-a10b");
        assert_eq!(resolved.model, "qwen/qwen3.5-122b-a10b");
    }

    #[test]
    fn keeps_legacy_aliases_resolving_to_cpa_ids() {
        assert_eq!(
            resolve_model_id(Some("cliproxyapi/qwen/qwen3.5-397b-a17b")),
            "cpa/qwen/qwen3.5-397b-a17b"
        );
        assert_eq!(
            resolve_model_id(Some("nvidia/qwen/qwen3.5-397b-a17b")),
            "cpa/qwen/qwen3.5-397b-a17b"
        );
    }

    #[test]
    fn accepts_cpa_env_aliases_before_cliproxyapi_envs() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        reset_env(&[
            "CPA_API_KEY",
            "CPA_BASE_URL",
            "CPA_API",
            "CLIPROXYAPI_API_KEY",
            "CLIPROXYAPI_BASE_URL",
            "CLIPROXYAPI_API",
            "OPENAI_API_KEY",
            "SAICODE_CONFIG_DIR",
            "CLAUDE_CONFIG_DIR",
        ]);

        env::set_var("CPA_API_KEY", "cpa-key");
        env::set_var("CPA_BASE_URL", "http://127.0.0.1:9999/v1");
        env::set_var("CPA_API", "openai-chat-completions");
        env::set_var("CLIPROXYAPI_API_KEY", "cliproxy-key");

        let provider = get_provider_config(&resolve_model(Some("cpa/qwen/qwen3.5-397b-a17b")))
            .expect("provider config should resolve");

        assert_eq!(provider.id, "cpa");
        assert_eq!(provider.api_key.as_deref(), Some("cpa-key"));
        assert_eq!(provider.base_url, "http://127.0.0.1:9999/v1");
        assert_eq!(format!("{:?}", provider.api), "OpenAIChatCompletions");
    }

    #[test]
    fn reads_runtime_config_from_saicode_config_dir() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        reset_env(&[
            "CPA_API_KEY",
            "CPA_BASE_URL",
            "CPA_API",
            "CLIPROXYAPI_API_KEY",
            "CLIPROXYAPI_BASE_URL",
            "CLIPROXYAPI_API",
            "OPENAI_API_KEY",
            "SAICODE_CONFIG_DIR",
            "CLAUDE_CONFIG_DIR",
        ]);

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let config_dir = env::temp_dir().join(format!("saicode-native-test-{unique}"));
        fs::create_dir_all(&config_dir).expect("create temp config dir");
        fs::write(
            config_dir.join("config.json"),
            r#"{"providers":{"cpa":{"api":"openai-chat-completions","baseUrl":"http://127.0.0.1:8317/v1","apiKey":"from-file"}}}"#,
        )
        .expect("write config");
        env::set_var("SAICODE_CONFIG_DIR", &config_dir);

        let provider = get_provider_config(&resolve_model(Some("cpa/gpt-5.4")))
            .expect("provider config should resolve");

        assert_eq!(provider.api_key.as_deref(), Some("from-file"));
        assert_eq!(provider.base_url, "http://127.0.0.1:8317/v1");

        let _ = fs::remove_file(config_dir.join("config.json"));
        let _ = fs::remove_dir(&config_dir);
    }
}
