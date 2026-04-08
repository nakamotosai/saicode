use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use api::{
    ApiError, ContentBlockDelta, InputMessage, MessageResponse, OpenAiCompatClient,
    OpenAiCompatConfig, OutputContentBlock, StreamEvent, Usage,
};
use runtime::{ConfigLoader, ProfileResolver, ProviderLauncher};
use saicode_core_adapter::{
    build_message_request, SaicodeEffortLevel, SaicodeModelSelection, SaicodeRequestEnvelope,
};
use serde_json::{json, Value};

const NO_CONTENT_MESSAGE: &str = "(no content)";
const DEFAULT_FAST_FALLBACK_WIRE_MODEL: &str = "gpt-5.4-mini";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Args {
    prompt: String,
    print: bool,
    model: Option<String>,
    effort: Option<SaicodeEffortLevel>,
    cwd: Option<PathBuf>,
    max_tokens: u32,
    output_format: OutputFormat,
    system_prompt: Option<String>,
    append_system_prompt: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args(env::args().skip(1).collect::<Vec<_>>())?;
    let cwd = args.cwd.clone().unwrap_or(env::current_dir()?);
    let runtime_config = ConfigLoader::default_for(&cwd).load()?;
    let resolved = ProfileResolver::resolve(&runtime_config, None, args.model.as_deref())?;
    let launch = ProviderLauncher::prepare(&resolved)?;
    let execution_model = preferred_wire_model_for_execution(&launch.model);

    let request = build_message_request(SaicodeRequestEnvelope {
        selection: SaicodeModelSelection {
            model: execution_model,
            effort: args.effort,
        },
        max_tokens: args.max_tokens,
        messages: vec![InputMessage::user_text(args.prompt)],
        system: join_system_prompt(
            args.system_prompt.as_deref(),
            args.append_system_prompt.as_deref(),
        ),
        tools: None,
        tool_choice: None,
        stream: true,
    });

    let client = OpenAiCompatClient::new(launch.api_key, OpenAiCompatConfig::saicode())
        .with_base_url(launch.base_url)
        .with_request_timeout(Duration::from_millis(launch.request_timeout_ms))
        .with_retry_policy(
            launch.max_retries,
            Duration::from_millis(200),
            Duration::from_secs(2),
        );
    let (response, text) =
        match stream_response(&client, &request, args.output_format == OutputFormat::Text).await {
            Ok(result) => result,
            Err(error) if should_retry_with_fast_model(&error, request.model.as_str()) => {
                let mut fallback_request = request.clone();
                fallback_request.model = DEFAULT_FAST_FALLBACK_WIRE_MODEL.to_string();
                stream_response(
                    &client,
                    &fallback_request,
                    args.output_format == OutputFormat::Text,
                )
                .await?
            }
            Err(error) => return Err(Box::new(error) as Box<dyn std::error::Error>),
        };

    match args.output_format {
        OutputFormat::Text => {
            if text.is_empty() {
                println!();
            }
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_json_output(&response, &text)?)
                    .map_err(|error| format!("failed to serialize response JSON: {error}"))?
            );
        }
    }

    Ok(())
}

async fn stream_response(
    client: &OpenAiCompatClient,
    request: &api::MessageRequest,
    stream_to_stdout: bool,
) -> Result<(MessageResponse, String), ApiError> {
    let mut stream = client.stream_message(request).await?;
    let mut response = None;
    let mut text = String::new();
    let mut usage = Usage {
        input_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
        output_tokens: 0,
    };

    while let Some(event) = stream.next_event().await? {
        match event {
            StreamEvent::MessageStart(start) => response = Some(start.message),
            StreamEvent::ContentBlockStart(start) => {
                if let OutputContentBlock::Text { text: initial } = start.content_block {
                    if !initial.is_empty() {
                        text.push_str(&initial);
                        if stream_to_stdout {
                            print!("{initial}");
                            io::stdout().flush()?;
                        }
                    }
                }
            }
            StreamEvent::ContentBlockDelta(delta) => {
                if let ContentBlockDelta::TextDelta { text: delta_text } = delta.delta {
                    if !delta_text.is_empty() {
                        text.push_str(&delta_text);
                        if stream_to_stdout {
                            print!("{delta_text}");
                            io::stdout().flush()?;
                        }
                    }
                }
            }
            StreamEvent::MessageDelta(delta) => usage = delta.usage,
            StreamEvent::MessageStop(_) => break,
            StreamEvent::ContentBlockStop(_) => {}
        }
    }

    let mut response =
        response.ok_or_else(|| io::Error::other("provider stream returned no message start"))?;
    response.usage = usage;
    response.content = if text.is_empty() {
        Vec::new()
    } else {
        vec![OutputContentBlock::Text { text: text.clone() }]
    };
    Ok((response, text))
}

fn should_retry_with_fast_model(error: &ApiError, wire_model: &str) -> bool {
    match error {
        ApiError::Api { body, .. } => {
            wire_model.eq_ignore_ascii_case("qwen/qwen3.5-122b-a10b")
                && is_degraded_function_invocation_error_text(body)
        }
        ApiError::RetriesExhausted { last_error, .. } => {
            should_retry_with_fast_model(last_error, wire_model)
        }
        _ => false,
    }
}

fn is_degraded_function_invocation_error_text(error_text: &str) -> bool {
    error_text.contains("Function id") && error_text.contains("DEGRADED function cannot be invoked")
}

fn preferred_wire_model_for_execution(wire_model: &str) -> String {
    if wire_model.eq_ignore_ascii_case("qwen/qwen3.5-122b-a10b") {
        return DEFAULT_FAST_FALLBACK_WIRE_MODEL.to_string();
    }

    wire_model.to_string()
}

fn parse_args(args: Vec<String>) -> Result<Args, String> {
    let mut prompt_parts = Vec::new();
    let mut print = false;
    let mut model = None;
    let mut effort = None;
    let mut cwd = None;
    let mut max_tokens = 1024;
    let mut output_format = OutputFormat::Text;
    let mut system_prompt = None;
    let mut append_system_prompt = None;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--" => {
                prompt_parts.extend(args[index + 1..].iter().cloned());
                break;
            }
            "-p" | "--print" => {
                print = true;
            }
            "--model" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--model requires a value".to_string())?;
                model = Some(value.clone());
            }
            "--effort" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--effort requires a value".to_string())?;
                effort = Some(parse_effort(value)?);
            }
            "--cwd" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--cwd requires a value".to_string())?;
                cwd = Some(PathBuf::from(value));
            }
            "--max-tokens" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--max-tokens requires a value".to_string())?;
                max_tokens = value
                    .parse::<u32>()
                    .map_err(|_| "--max-tokens must be a positive integer".to_string())?;
            }
            "--output-format" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--output-format requires a value".to_string())?;
                output_format = parse_output_format(value)?;
            }
            "--system-prompt" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--system-prompt requires a value".to_string())?;
                system_prompt = Some(value.clone());
            }
            "--system-prompt-file" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--system-prompt-file requires a value".to_string())?;
                system_prompt = Some(
                    fs::read_to_string(value)
                        .map_err(|error| format!("failed to read system prompt file: {error}"))?,
                );
            }
            "--append-system-prompt" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--append-system-prompt requires a value".to_string())?;
                append_system_prompt = Some(value.clone());
            }
            "--append-system-prompt-file" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--append-system-prompt-file requires a value".to_string())?;
                append_system_prompt = Some(fs::read_to_string(value).map_err(|error| {
                    format!("failed to read append system prompt file: {error}")
                })?);
            }
            "--bare"
            | "--dangerously-skip-permissions"
            | "--allow-dangerously-skip-permissions" => {}
            "--help" | "-h" => return Err(usage()),
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}\n\n{}", usage()));
            }
            value => prompt_parts.push(value.to_string()),
        }
        index += 1;
    }

    let prompt = if prompt_parts.is_empty() {
        read_prompt_from_stdin()?
    } else {
        prompt_parts.join(" ").trim().to_string()
    };

    if prompt.is_empty() {
        return Err(format!("prompt is required\n\n{}", usage()));
    }

    Ok(Args {
        prompt,
        print,
        model,
        effort,
        cwd,
        max_tokens,
        output_format,
        system_prompt,
        append_system_prompt,
    })
}

fn parse_effort(value: &str) -> Result<SaicodeEffortLevel, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "low" => Ok(SaicodeEffortLevel::Low),
        "medium" => Ok(SaicodeEffortLevel::Medium),
        "high" => Ok(SaicodeEffortLevel::High),
        "max" | "xhigh" => Ok(SaicodeEffortLevel::Max),
        _ => Err("--effort must be one of: low, medium, high, max, xhigh".to_string()),
    }
}

fn parse_output_format(value: &str) -> Result<OutputFormat, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        _ => Err("--output-format must be one of: text, json".to_string()),
    }
}

fn join_system_prompt(
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

fn read_prompt_from_stdin() -> Result<String, String> {
    if io::stdin().is_terminal() {
        return Ok(String::new());
    }

    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    Ok(buffer.trim().to_string())
}

fn build_json_output(response: &MessageResponse, text: &str) -> Result<Value, String> {
    let timestamp = synthetic_timestamp()?;
    let top_level_id = synthetic_uuid_like("turn")?;
    let message_id = if response.id.trim().is_empty() {
        synthetic_uuid_like("msg")?
    } else {
        response.id.clone()
    };

    Ok(json!({
        "type": "assistant",
        "uuid": top_level_id,
        "timestamp": timestamp,
        "message": {
            "id": message_id,
            "container": Value::Null,
            "model": response.model,
            "role": response.role,
            "stop_reason": response.stop_reason.clone().unwrap_or_else(|| "end_turn".to_string()),
            "stop_sequence": response.stop_sequence.clone().unwrap_or_default(),
            "type": response.kind,
            "usage": {
                "input_tokens": response.usage.input_tokens,
                "output_tokens": response.usage.output_tokens,
                "cache_creation_input_tokens": response.usage.cache_creation_input_tokens,
                "cache_read_input_tokens": response.usage.cache_read_input_tokens,
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
        .map_err(|error| format!("clock error: {error}"))?;
    Ok(now.as_secs().to_string())
}

fn synthetic_uuid_like(prefix: &str) -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("clock error: {error}"))?;
    Ok(format!("{prefix}-{:x}", now.as_nanos()))
}

fn usage() -> String {
    [
        "Usage: saicode-rust-one-shot [options] [prompt]",
        "",
        "Compatible simple print path for saicode.",
        "",
        "Options:",
        "  -p, --print                 Accept saicode print-mode invocations",
        "  --model <model>            Override model",
        "  --effort <level>           low | medium | high | max | xhigh",
        "  --cwd <path>               Run config resolution from a specific directory",
        "  --max-tokens <n>           Max output tokens (default: 1024)",
        "  --output-format <format>   text | json",
        "  --system-prompt <text>     Override system prompt",
        "  --system-prompt-file <f>   Read system prompt from file",
        "  --append-system-prompt <t> Append to system prompt",
        "  --append-system-prompt-file <f> Read appended system prompt from file",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        parse_args, parse_effort, preferred_wire_model_for_execution, should_retry_with_fast_model,
        Args, OutputFormat,
    };
    use api::ApiError;
    use saicode_core_adapter::SaicodeEffortLevel;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_saicode_compatible_prompt_and_flags() {
        let args = parse_args(vec![
            "-p".to_string(),
            "--model".to_string(),
            "gpt-5.4".to_string(),
            "--effort".to_string(),
            "xhigh".to_string(),
            "--cwd".to_string(),
            "/tmp/project".to_string(),
            "--output-format".to_string(),
            "json".to_string(),
            "hello".to_string(),
            "world".to_string(),
        ])
        .expect("args should parse");

        assert_eq!(
            args,
            Args {
                prompt: "hello world".to_string(),
                print: true,
                model: Some("gpt-5.4".to_string()),
                effort: Some(SaicodeEffortLevel::Max),
                cwd: Some(PathBuf::from("/tmp/project")),
                max_tokens: 1024,
                output_format: OutputFormat::Json,
                system_prompt: None,
                append_system_prompt: None,
            }
        );
    }

    #[test]
    fn parses_system_prompt_files() {
        let temp_path = std::env::temp_dir().join(format!(
            "saicode-rust-one-shot-test-{}.txt",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::write(&temp_path, "be concise").expect("write test file");

        let args = parse_args(vec![
            "--system-prompt-file".to_string(),
            temp_path.display().to_string(),
            "hello".to_string(),
        ])
        .expect("args should parse");

        assert_eq!(args.system_prompt.as_deref(), Some("be concise"));

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn rejects_missing_prompt() {
        let error = parse_args(vec!["--model".to_string(), "gpt-5.4".to_string()])
            .expect_err("missing prompt should error");
        assert!(error.contains("prompt is required"));
    }

    #[test]
    fn parses_double_dash_prompt_separator() {
        let args = parse_args(vec![
            "-p".to_string(),
            "--output-format".to_string(),
            "text".to_string(),
            "--".to_string(),
            "hello".to_string(),
            "world".to_string(),
        ])
        .expect("args should parse");

        assert_eq!(args.prompt, "hello world");
        assert!(args.print);
    }

    #[test]
    fn parses_effort_aliases() {
        assert_eq!(parse_effort("low").expect("low"), SaicodeEffortLevel::Low);
        assert_eq!(
            parse_effort("xhigh").expect("xhigh"),
            SaicodeEffortLevel::Max
        );
    }

    #[test]
    fn retries_qwen_degraded_errors_with_fast_model() {
        let error = ApiError::Api {
            status: "400".parse().expect("status code"),
            error_type: None,
            message: None,
            body: "Function id 'abc': DEGRADED function cannot be invoked".to_string(),
            retryable: false,
        };

        assert!(should_retry_with_fast_model(
            &error,
            "qwen/qwen3.5-122b-a10b"
        ));
        assert!(!should_retry_with_fast_model(&error, "gpt-5.4-mini"));
    }

    #[test]
    fn prefers_fast_execution_model_for_qwen_baseline() {
        assert_eq!(
            preferred_wire_model_for_execution("qwen/qwen3.5-122b-a10b"),
            "gpt-5.4-mini"
        );
        assert_eq!(
            preferred_wire_model_for_execution("gpt-5.4-mini"),
            "gpt-5.4-mini"
        );
    }
}
