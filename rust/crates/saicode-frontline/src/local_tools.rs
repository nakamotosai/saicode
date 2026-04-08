use crate::recovery::{
    build_json_output_from_text, execute_provider_chat_completions_stream,
    execute_provider_json_request, extract_usage, get_provider_config, join_system_prompt,
    read_prompt_from_stdin, resolve_model, OutputFormat, ProviderConfig, ResolvedModel, Usage,
    WireApi,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SUPPORTED_TOOL_NAMES: &[&str] = &[
    "Read",
    "Grep",
    "Glob",
    "Bash",
    "Write",
    "Edit",
    "WebSearch",
    "WebFetch",
];
const DEFAULT_MAX_TURNS: usize = 8;
const DEFAULT_READ_LIMIT: usize = 2_000;
const DEFAULT_GREP_LIMIT: usize = 250;
const DEFAULT_GLOB_LIMIT: usize = 100;
const MAX_EDIT_FILE_SIZE: u64 = 1024 * 1024 * 1024;
const DEFAULT_BASH_TIMEOUT_MS: usize = 15_000;
const MAX_BASH_TIMEOUT_MS: usize = 30_000;
const DEFAULT_BASH_OUTPUT_LIMIT: usize = 24_000;
const DEFAULT_WEB_SEARCH_TIMEOUT_MS: u64 = 20_000;
const DEFAULT_WEB_SEARCH_SAI_TIMEOUT_MS: u64 = 45_000;
const DEFAULT_PAGE_FETCH_TIMEOUT_MS: u64 = 12_000;
const DEFAULT_WEB_FETCH_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_MAX_REDIRECTS: usize = 10;
const DEFAULT_MAX_SEARCH_RESULTS: usize = 8;
const DEFAULT_MAX_FETCHED_PAGES: usize = 2;
const DEFAULT_PAGE_TEXT_LENGTH: usize = 3_000;
const DEFAULT_EXCERPT_LENGTH: usize = 500;
const DEFAULT_WEB_FETCH_MARKDOWN_LENGTH: usize = 100_000;
const MAX_HTTP_CONTENT_LENGTH: usize = 10 * 1024 * 1024;
const DEFAULT_SEARCH_BASE_URL: &str = "https://html.duckduckgo.com/html/";
const DEFAULT_LOCAL_SAI_SEARCH_BASE_URL: &str = "http://127.0.0.1:18961";
const DEFAULT_SAI_SEARCH_SSH_TARGET: &str = "ubuntu@vps-jp.tail4b5213.ts.net";
const DEFAULT_SAI_SEARCH_SSH_REMOTE_URL: &str = "http://127.0.0.1:18961/search";
const SEARCH_USER_AGENT: &str =
    "Mozilla/5.0 (compatible; saicode-web-search-native/1.0; +https://saicode.local)";
const WEB_FETCH_USER_AGENT: &str =
    "Mozilla/5.0 (compatible; saicode-web-fetch-native/1.0; +https://saicode.local)";
const DEFAULT_SMALL_FAST_MODEL_ID: &str = "cpa/gpt-5.4-mini";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeLocalToolsOutcome {
    Completed,
    FallbackToRustFullCli(String),
}

#[derive(Clone, Debug)]
struct ParsedLocalToolsArgs {
    print: bool,
    model: Option<String>,
    system_prompt: Option<String>,
    append_system_prompt: Option<String>,
    output_format: OutputFormat,
    max_turns: usize,
    permission_mode: Option<String>,
    dangerously_skip_permissions: bool,
    tools: Vec<String>,
    prompt: String,
}

#[derive(Clone, Debug)]
struct ToolCall {
    id: String,
    name: String,
    input: Value,
}

#[derive(Clone, Debug)]
enum ConversationMessage {
    UserText(String),
    Assistant {
        text: String,
        tool_calls: Vec<ToolCall>,
    },
    ToolResult {
        call_id: String,
        output: String,
    },
}

#[derive(Clone, Debug)]
struct AssistantTurn {
    text: String,
    tool_calls: Vec<ToolCall>,
    usage: Usage,
}

enum ToolExecution {
    Output(String),
    FallbackToRustFullCli(String),
}

#[derive(Clone, Debug)]
struct NativeReadSnapshot {
    content: String,
    timestamp_ms: u64,
    is_partial_view: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeTextEncoding {
    Utf8,
    Utf16Le,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeLineEnding {
    Lf,
    Crlf,
}

#[derive(Clone, Debug)]
struct NativeTextFile {
    content: String,
    encoding: NativeTextEncoding,
    line_endings: NativeLineEnding,
    timestamp_ms: u64,
}

#[derive(Clone, Debug)]
struct NativeToolSession {
    cwd: PathBuf,
    read_snapshots: BTreeMap<PathBuf, NativeReadSnapshot>,
    bash_policy: NativeBashPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeBashPolicy {
    ReadOnly,
    Unrestricted,
}

#[derive(Clone, Debug)]
struct SearchHit {
    title: String,
    url: String,
}

#[derive(Clone, Debug)]
struct FetchedPage {
    title: String,
    url: String,
    excerpt: String,
}

#[derive(Clone, Debug)]
struct NativeWebSearchOutput {
    query: String,
    hits: Vec<SearchHit>,
    fetched_pages: Vec<FetchedPage>,
}

#[derive(Clone, Debug)]
struct HeaderLine {
    name: String,
    value: String,
}

#[derive(Clone, Debug)]
struct HttpFetchResponse {
    status_code: u16,
    content_type: String,
    headers: Vec<HeaderLine>,
    body: Vec<u8>,
}

#[derive(Clone, Debug)]
struct RedirectInfo {
    original_url: String,
    redirect_url: String,
    status_code: u16,
}

#[derive(Clone, Debug)]
enum NativeWebFetchResponse {
    Redirect(RedirectInfo),
    Content {
        content: String,
        content_type: String,
    },
}

#[derive(Clone, Debug)]
struct SimpleUrl {
    scheme: String,
    host: String,
    port: Option<String>,
    path_and_query: String,
}

impl NativeToolSession {
    fn new(cwd: PathBuf, bash_policy: NativeBashPolicy) -> Self {
        Self {
            cwd,
            read_snapshots: BTreeMap::new(),
            bash_policy,
        }
    }

    fn display_path(&self, path: &Path) -> String {
        relativize_display_path(&self.cwd, path)
    }

    fn snapshot(&self, path: &Path) -> Option<&NativeReadSnapshot> {
        self.read_snapshots.get(path)
    }

    fn update_snapshot(
        &mut self,
        path: &Path,
        content: String,
        timestamp_ms: u64,
        is_partial: bool,
    ) {
        self.read_snapshots.insert(
            path.to_path_buf(),
            NativeReadSnapshot {
                content,
                timestamp_ms,
                is_partial_view: is_partial,
            },
        );
    }

    fn clear_snapshots(&mut self) {
        self.read_snapshots.clear();
    }
}

pub fn should_handle_natively(args: &[String]) -> bool {
    if is_env_truthy(
        env::var("SAICODE_DISABLE_NATIVE_LOCAL_TOOLS")
            .ok()
            .as_deref(),
    ) {
        return false;
    }

    matches!(
        parse_local_tools_args(args),
        Ok(parsed) if parsed.print && !parsed.tools.is_empty()
    )
}

pub fn run_native_local_tools(args: &[String]) -> Result<NativeLocalToolsOutcome, String> {
    let parsed = parse_local_tools_args(args)?;

    if !parsed.print {
        return Err("Native local-tools path only handles --print requests".to_string());
    }

    let prompt = if parsed.prompt.trim().is_empty() {
        read_prompt_from_stdin()?
    } else {
        parsed.prompt.clone()
    };
    if prompt.trim().is_empty() {
        return Err("Error: prompt is required".to_string());
    }

    let cwd = canonical_cwd()?;
    let mut session = NativeToolSession::new(cwd.clone(), determine_bash_policy(&parsed));
    let resolved_model = resolve_native_local_tools_model(parsed.model.as_deref());
    let provider = get_provider_config(&resolved_model)?;
    let user_system_prompt = join_system_prompt(
        parsed.system_prompt.as_deref(),
        parsed.append_system_prompt.as_deref(),
    );
    let native_instruction = native_system_prompt(&cwd, &parsed.tools, session.bash_policy);
    let effective_system_prompt = match user_system_prompt {
        Some(user) => Some(format!(
            "{native_instruction}\n\nAdditional system instructions:\n{user}"
        )),
        None => Some(native_instruction),
    };

    let mut messages = vec![ConversationMessage::UserText(prompt)];
    let mut total_usage = Usage::default();

    for _ in 0..parsed.max_turns {
        let body = build_request_body(
            &provider,
            &resolved_model,
            effective_system_prompt.as_deref(),
            &messages,
            &parsed.tools,
            session.bash_policy,
        );
        let turn = if provider.api == WireApi::OpenAIChatCompletions {
            parse_streamed_assistant_turn(execute_provider_chat_completions_stream(
                &provider, &body, false,
            )?)
        } else {
            let response = execute_provider_json_request(&provider, &body)?;
            parse_assistant_turn(&response, provider.api)?
        };
        total_usage.input_tokens += turn.usage.input_tokens;
        total_usage.output_tokens += turn.usage.output_tokens;

        if turn.tool_calls.is_empty() {
            emit_final_output(&turn.text, total_usage, parsed.output_format)?;
            return Ok(NativeLocalToolsOutcome::Completed);
        }

        messages.push(ConversationMessage::Assistant {
            text: turn.text,
            tool_calls: turn.tool_calls.clone(),
        });

        for tool_call in turn.tool_calls {
            match execute_tool_call(&tool_call, &mut session) {
                ToolExecution::Output(output) => {
                    messages.push(ConversationMessage::ToolResult {
                        call_id: tool_call.id,
                        output,
                    });
                }
                ToolExecution::FallbackToRustFullCli(reason) => {
                    return Ok(NativeLocalToolsOutcome::FallbackToRustFullCli(reason));
                }
            }
        }
    }

    Err(format!(
        "Native local-tools path exceeded max turns ({}) without a final answer",
        parsed.max_turns
    ))
}

fn emit_final_output(text: &str, usage: Usage, output_format: OutputFormat) -> Result<(), String> {
    match output_format {
        OutputFormat::Text => {
            println!("{text}");
        }
        OutputFormat::Json => {
            let payload = build_json_output_from_text(text, usage)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&payload)
                    .map_err(|error| format!("Failed to render JSON output: {error}"))?
            );
        }
    }

    Ok(())
}

fn parse_local_tools_args(args: &[String]) -> Result<ParsedLocalToolsArgs, String> {
    let mut parsed = ParsedLocalToolsArgs {
        print: false,
        model: None,
        system_prompt: None,
        append_system_prompt: None,
        output_format: OutputFormat::Text,
        max_turns: DEFAULT_MAX_TURNS,
        permission_mode: None,
        dangerously_skip_permissions: false,
        tools: Vec::new(),
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
            "-p" | "--print" => {
                parsed.print = true;
            }
            "--bare" | "--allow-dangerously-skip-permissions" => {}
            "--dangerously-skip-permissions" => {
                parsed.dangerously_skip_permissions = true;
            }
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
            "--append-system-prompt-file" => {
                let path = read_flag_value(args, index, "--append-system-prompt-file")?;
                parsed.append_system_prompt = Some(
                    fs::read_to_string(&path)
                        .map_err(|error| format!("Failed to read {}: {error}", path))?,
                );
                index += 1;
            }
            "--output-format" => {
                let value = read_flag_value(args, index, "--output-format")?;
                parsed.output_format = match value.as_str() {
                    "json" => OutputFormat::Json,
                    "text" => OutputFormat::Text,
                    _ => {
                        return Err(format!(
                            "Unsupported output format for native local-tools path: {value}"
                        ))
                    }
                };
                index += 1;
            }
            "--max-turns" => {
                let value = read_flag_value(args, index, "--max-turns")?;
                parsed.max_turns = parse_positive_integer("--max-turns", &value)?;
                index += 1;
            }
            "--permission-mode" => {
                parsed.permission_mode = Some(read_flag_value(args, index, "--permission-mode")?);
                index += 1;
            }
            "--tools" | "--allowedTools" | "--allowed-tools" => {
                let (values, next_index) = collect_variadic_option_values(args, index);
                if contains_bash_rule_suffix(&values) {
                    return Err(
                        "Native local-tools path does not support Bash(...) permission matcher rules"
                            .to_string(),
                    );
                }
                parsed
                    .tools
                    .extend(normalize_tool_restriction_values(&values));
                index = next_index;
            }
            _ if arg.starts_with('-') => {
                return Err(format!(
                    "Unsupported flag for native local-tools path: {arg}"
                ));
            }
            _ => positional.push(args[index].clone()),
        }

        index += 1;
    }

    let mut deduped = BTreeSet::new();
    parsed.tools.retain(|tool| deduped.insert(tool.clone()));
    if !parsed.tools.iter().all(|tool| {
        SUPPORTED_TOOL_NAMES
            .iter()
            .any(|candidate| candidate == tool)
    }) {
        return Err(
            "Native local-tools path only supports Read/Grep/Glob/Bash/Write/Edit/WebSearch/WebFetch"
                .to_string(),
        );
    }

    parsed.prompt = positional.join(" ").trim().to_string();
    Ok(parsed)
}

fn determine_bash_policy(parsed: &ParsedLocalToolsArgs) -> NativeBashPolicy {
    if !parsed.tools.iter().any(|tool| tool == "Bash") {
        return NativeBashPolicy::ReadOnly;
    }

    if parsed.dangerously_skip_permissions
        || parsed.permission_mode.as_deref() == Some("bypassPermissions")
    {
        NativeBashPolicy::Unrestricted
    } else {
        NativeBashPolicy::ReadOnly
    }
}

fn resolve_native_local_tools_model(cli_model: Option<&str>) -> ResolvedModel {
    if let Some(model) = cli_model.filter(|value| !value.trim().is_empty()) {
        return resolve_model(Some(model));
    }

    let fast_model = env::var("SAICODE_NATIVE_LOCAL_TOOLS_MODEL")
        .ok()
        .or_else(|| env::var("SAICODE_SMALL_FAST_MODEL").ok())
        .or_else(|| env::var("SAICODE_DEFAULT_HAIKU_MODEL").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SMALL_FAST_MODEL_ID.to_string());

    resolve_model(Some(&fast_model))
}

fn contains_bash_rule_suffix(values: &[String]) -> bool {
    values
        .iter()
        .flat_map(|value| {
            value
                .split(|ch: char| ch == ',' || ch.is_whitespace())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        })
        .any(|item| item.starts_with("Bash("))
}

fn read_flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .filter(|value| !value.starts_with('-'))
        .cloned()
        .ok_or_else(|| format!("Missing value for {flag}"))
}

fn parse_positive_integer(flag: &str, value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{flag} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{flag} must be a positive integer"));
    }
    Ok(parsed.min(16))
}

fn collect_variadic_option_values(cli_args: &[String], start_index: usize) -> (Vec<String>, usize) {
    let mut values = Vec::new();
    let mut index = start_index + 1;

    while index < cli_args.len() {
        let value = cli_args[index].trim();
        if value.is_empty() {
            index += 1;
            continue;
        }
        if value.starts_with('-') {
            break;
        }
        values.push(cli_args[index].clone());
        index += 1;
    }

    let next_index = if index == start_index + 1 {
        start_index
    } else {
        index - 1
    };

    (values, next_index)
}

fn normalize_tool_restriction_values(values: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for value in values {
        for part in value.split(|ch: char| ch == ',' || ch.is_whitespace()) {
            let item = part.trim();
            if item.is_empty() {
                continue;
            }
            let without_suffix = item.split('(').next().unwrap_or(item);
            normalized.push(without_suffix.to_string());
        }
    }
    normalized
}

fn native_system_prompt(cwd: &Path, tools: &[String], bash_policy: NativeBashPolicy) -> String {
    let mut guidance = vec![
        "You are saicode native local-tools mode for one-shot print requests.".to_string(),
        "Use the provided tools when local repository state is needed. Do not guess about local files.".to_string(),
        format!("Current working directory: {}", cwd.display()),
        format!("Available tools: {}", tools.join(", ")),
        "Tool guidance:".to_string(),
        "- Use Grep to search file contents.".to_string(),
        "- Use Glob to find files by path pattern.".to_string(),
        "- Use Read to inspect specific files.".to_string(),
    ];

    if tools.iter().any(|tool| tool == "Write" || tool == "Edit") {
        guidance.push(
            "- Read an existing file fully before modifying it with Write or Edit.".to_string(),
        );
        guidance.push(
            "- Write replaces the entire file content. Use it for new files or full rewrites."
                .to_string(),
        );
        guidance.push(
            "- Edit replaces exact text in an existing file. Use replace_all only when you really want every match changed."
                .to_string(),
        );
    }

    if tools.iter().any(|tool| tool == "Bash") {
        match bash_policy {
            NativeBashPolicy::ReadOnly => {
                guidance.push("- Bash is limited to readonly inspection commands such as pwd, ls, command -v, and readonly git status/log/show/diff-style checks.".to_string());
                guidance.push("- Do not use Bash for writes, installs, background tasks, redirections, or shell scripting tricks.".to_string());
                guidance.push(
                    "- Prefer Read/Grep/Glob over Bash when a dedicated file tool is enough."
                        .to_string(),
                );
            }
            NativeBashPolicy::Unrestricted => {
                guidance.push("- Bash can run general shell commands directly in the current working directory because this session already bypasses permission prompts.".to_string());
                guidance.push("- Background execution is still unsupported here; keep commands foreground and bounded by timeout.".to_string());
                guidance.push("- Prefer Read/Grep/Glob/Write/Edit when a dedicated file tool is enough, but you may use Bash for builds, tests, installs, and shell file operations.".to_string());
            }
        }
    }
    if tools.iter().any(|tool| tool == "WebSearch") {
        guidance.push(
            "- WebSearch performs a real web search and returns live links plus short excerpts from top pages."
                .to_string(),
        );
        guidance.push(
            "- When you use WebSearch, include a Sources section in the final answer with markdown links."
                .to_string(),
        );
    }
    if tools.iter().any(|tool| tool == "WebFetch") {
        guidance.push(
            "- WebFetch fetches a public webpage and applies the provided prompt to the fetched content."
                .to_string(),
        );
        guidance.push(
            "- If WebFetch reports a redirect to a different host, call WebFetch again on the returned redirect URL."
                .to_string(),
        );
    }

    guidance.push(
        "When you have enough information, answer the user directly and concisely in the user's language."
            .to_string(),
    );

    guidance.join("\n")
}

fn build_request_body(
    provider: &ProviderConfig,
    resolved_model: &ResolvedModel,
    system_prompt: Option<&str>,
    messages: &[ConversationMessage],
    tools: &[String],
    bash_policy: NativeBashPolicy,
) -> Value {
    let tool_defs = build_openai_tools(provider.api, tools, bash_policy);
    match provider.api {
        WireApi::OpenAIResponses => {
            let mut body = json!({
                "model": resolved_model.model,
                "input": convert_messages_to_responses_input(messages),
                "tools": tool_defs,
                "parallel_tool_calls": false,
                "max_output_tokens": resolved_model.max_output_tokens,
            });
            if let Some(system) = system_prompt {
                body["instructions"] = Value::String(system.to_string());
            }
            body
        }
        WireApi::OpenAIChatCompletions => {
            let mut body = json!({
                "model": resolved_model.model,
                "messages": convert_messages_to_chat_completions(messages, system_prompt),
                "tools": tool_defs,
                "tool_choice": "auto",
                "max_tokens": resolved_model.max_output_tokens,
            });
            if tools.is_empty() {
                body.as_object_mut().map(|object| {
                    object.remove("tools");
                    object.remove("tool_choice");
                });
            }
            body
        }
    }
}

fn build_openai_tools(api: WireApi, tools: &[String], bash_policy: NativeBashPolicy) -> Value {
    let defs: Vec<Value> = tools
        .iter()
        .filter_map(|tool| tool_definition(tool, bash_policy))
        .map(|definition| match api {
            WireApi::OpenAIResponses => json!({
                "type": "function",
                "name": definition.name,
                "description": definition.description,
                "parameters": definition.parameters,
            }),
            WireApi::OpenAIChatCompletions => json!({
                "type": "function",
                "function": {
                    "name": definition.name,
                    "description": definition.description,
                    "parameters": definition.parameters,
                }
            }),
        })
        .collect();
    Value::Array(defs)
}

struct ToolDefinition {
    name: &'static str,
    description: &'static str,
    parameters: Value,
}

fn tool_definition(name: &str, bash_policy: NativeBashPolicy) -> Option<ToolDefinition> {
    match name {
        "Read" => Some(ToolDefinition {
            name: "Read",
            description: "Read a text file from the local filesystem. Use this for exact file contents.",
            parameters: json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "file_path": { "type": "string", "description": "Absolute or cwd-relative path to the file to read" },
                    "offset": { "type": "integer", "minimum": 0, "description": "Optional starting line number (1-based). Omit to start at the beginning." },
                    "limit": { "type": "integer", "minimum": 1, "description": "Optional number of lines to read. Defaults to 2000." },
                    "pages": { "type": "string", "description": "PDF page range. Not supported in native path yet." }
                },
                "required": ["file_path"]
            }),
        }),
        "Grep" => Some(ToolDefinition {
            name: "Grep",
            description: "Search file contents with ripgrep. Use this for locating strings or regex matches.",
            parameters: json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "pattern": { "type": "string", "description": "Regex pattern to search for" },
                    "path": { "type": "string", "description": "Optional cwd-relative directory or file to search in" },
                    "glob": { "type": "string", "description": "Optional glob filter such as *.ts or src/**/*.rs" },
                    "output_mode": { "type": "string", "enum": ["content", "files_with_matches", "count"], "description": "Search output mode" },
                    "-B": { "type": "integer", "minimum": 0, "description": "Show lines before each match" },
                    "-A": { "type": "integer", "minimum": 0, "description": "Show lines after each match" },
                    "-C": { "type": "integer", "minimum": 0, "description": "Show surrounding context lines" },
                    "context": { "type": "integer", "minimum": 0, "description": "Alias for -C" },
                    "-n": { "type": "boolean", "description": "Show line numbers. Defaults to true for content mode." },
                    "-i": { "type": "boolean", "description": "Case-insensitive search" },
                    "type": { "type": "string", "description": "Optional ripgrep file type filter such as rust, ts, py" },
                    "head_limit": { "type": "integer", "minimum": 0, "description": "Maximum number of result lines or entries. Defaults to 250. Use 0 for unlimited." },
                    "offset": { "type": "integer", "minimum": 0, "description": "Skip this many result lines or entries before returning output" },
                    "multiline": { "type": "boolean", "description": "Enable multiline matching" }
                },
                "required": ["pattern"]
            }),
        }),
        "Glob" => Some(ToolDefinition {
            name: "Glob",
            description: "Find files by glob pattern. Use this when you need file paths rather than file contents.",
            parameters: json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "pattern": { "type": "string", "description": "Glob pattern to match, such as **/*.ts" },
                    "path": { "type": "string", "description": "Optional cwd-relative directory to search in" }
                },
                "required": ["pattern"]
            }),
        }),
        "Bash" => Some(ToolDefinition {
            name: "Bash",
            description: match bash_policy {
                NativeBashPolicy::ReadOnly => "Run a readonly shell command for local inspection. Only use this for safe, non-mutating checks such as pwd, ls, command -v, or readonly git inspection.",
                NativeBashPolicy::Unrestricted => "Run a shell command in the current working directory. This session already bypasses permission prompts, so write-capable commands are allowed here. Background execution is still unsupported.",
            },
            parameters: json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "command": { "type": "string", "description": "Readonly shell command to run" },
                    "timeout": { "type": "integer", "minimum": 1, "description": "Optional timeout in milliseconds. Defaults to 15000 and is capped at 30000." },
                    "description": { "type": "string", "description": "Optional plain-language description of the command" },
                    "run_in_background": { "type": "boolean", "description": "Not supported in native mode." },
                    "dangerouslyDisableSandbox": { "type": "boolean", "description": "Not supported in native mode." }
                },
                "required": ["command"]
            }),
        }),
        "Write" => Some(ToolDefinition {
            name: "Write",
            description: "Create a new text file or fully overwrite an existing text file within the current working directory. Existing files must be read first.",
            parameters: json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "file_path": { "type": "string", "description": "Absolute or cwd-relative path to the file to write" },
                    "content": { "type": "string", "description": "The full file content to write" }
                },
                "required": ["file_path", "content"]
            }),
        }),
        "Edit" => Some(ToolDefinition {
            name: "Edit",
            description: "Modify a text file by replacing exact text within the current working directory. Existing files must be read first.",
            parameters: json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "file_path": { "type": "string", "description": "Absolute or cwd-relative path to the file to edit" },
                    "old_string": { "type": "string", "description": "The exact text to replace" },
                    "new_string": { "type": "string", "description": "The replacement text" },
                    "replace_all": { "type": "boolean", "description": "Replace all matches instead of just one. Defaults to false." }
                },
                "required": ["file_path", "old_string", "new_string"]
            }),
        }),
        "WebSearch" => Some(ToolDefinition {
            name: "WebSearch",
            description: "Search the web for current information. Returns markdown links and short excerpts from top pages.",
            parameters: json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "query": { "type": "string", "description": "The search query to use" },
                    "allowed_domains": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Only include results from these domains"
                    },
                    "blocked_domains": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Never include results from these domains"
                    }
                },
                "required": ["query"]
            }),
        }),
        "WebFetch" => Some(ToolDefinition {
            name: "WebFetch",
            description: "Fetch a public webpage and apply the provided prompt to the fetched content.",
            parameters: json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "url": { "type": "string", "description": "The URL to fetch content from" },
                    "prompt": { "type": "string", "description": "The prompt to run on the fetched content" }
                },
                "required": ["url", "prompt"]
            }),
        }),
        _ => None,
    }
}

fn convert_messages_to_responses_input(messages: &[ConversationMessage]) -> Value {
    let mut input = Vec::new();
    for message in messages {
        match message {
            ConversationMessage::UserText(text) => input.push(json!({
                "role": "user",
                "content": [{ "type": "input_text", "text": text }],
            })),
            ConversationMessage::Assistant { text, tool_calls } => {
                for tool_call in tool_calls {
                    input.push(json!({
                        "type": "function_call",
                        "call_id": tool_call.id,
                        "name": tool_call.name,
                        "arguments": tool_call.input.to_string(),
                    }));
                }
                if !text.is_empty() {
                    input.push(json!({
                        "role": "assistant",
                        "content": [{ "type": "output_text", "text": text }],
                    }));
                }
            }
            ConversationMessage::ToolResult { call_id, output } => {
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": call_id,
                    "output": output,
                }));
            }
        }
    }
    Value::Array(input)
}

fn convert_messages_to_chat_completions(
    messages: &[ConversationMessage],
    system_prompt: Option<&str>,
) -> Value {
    let mut out = Vec::new();
    if let Some(system) = system_prompt {
        out.push(json!({ "role": "system", "content": system }));
    }

    for message in messages {
        match message {
            ConversationMessage::UserText(text) => {
                out.push(json!({ "role": "user", "content": text }));
            }
            ConversationMessage::Assistant { text, tool_calls } => {
                let tool_calls_json: Vec<Value> = tool_calls
                    .iter()
                    .map(|tool_call| {
                        json!({
                            "id": tool_call.id,
                            "type": "function",
                            "function": {
                                "name": tool_call.name,
                                "arguments": tool_call.input.to_string(),
                            }
                        })
                    })
                    .collect();
                out.push(json!({
                    "role": "assistant",
                    "content": text,
                    "tool_calls": tool_calls_json,
                }));
            }
            ConversationMessage::ToolResult { call_id, output } => {
                out.push(json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": output,
                }));
            }
        }
    }

    Value::Array(out)
}

fn parse_assistant_turn(json: &Value, api: WireApi) -> Result<AssistantTurn, String> {
    match api {
        WireApi::OpenAIResponses => parse_responses_assistant_turn(json),
        WireApi::OpenAIChatCompletions => parse_chat_assistant_turn(json),
    }
}

fn parse_streamed_assistant_turn(
    streamed: crate::recovery::StreamedChatCompletion,
) -> AssistantTurn {
    AssistantTurn {
        text: streamed.text,
        tool_calls: streamed
            .tool_calls
            .into_iter()
            .map(|tool_call| ToolCall {
                id: tool_call.id,
                name: tool_call.name,
                input: tool_call.input,
            })
            .collect(),
        usage: streamed.usage,
    }
}

fn parse_responses_assistant_turn(json: &Value) -> Result<AssistantTurn, String> {
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();

    if let Some(items) = json.get("output").and_then(Value::as_array) {
        for item in items {
            if item.get("type").and_then(Value::as_str) == Some("message") {
                if let Some(content) = item.get("content").and_then(Value::as_array) {
                    for block in content {
                        if block.get("type").and_then(Value::as_str) == Some("output_text") {
                            if let Some(text) = block.get("text").and_then(Value::as_str) {
                                text_parts.push(text.to_string());
                            }
                        }
                    }
                }
                continue;
            }

            if item.get("type").and_then(Value::as_str) == Some("function_call") {
                tool_calls.push(ToolCall {
                    id: item
                        .get("call_id")
                        .or_else(|| item.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or("native-tool-call")
                        .to_string(),
                    name: item
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    input: parse_tool_arguments(item.get("arguments")),
                });
            }
        }
    }

    if text_parts.is_empty() {
        if let Some(text) = json.get("output_text").and_then(Value::as_str) {
            if !text.is_empty() {
                text_parts.push(text.to_string());
            }
        }
    }

    Ok(AssistantTurn {
        text: text_parts.join("\n\n"),
        tool_calls,
        usage: extract_usage(json, WireApi::OpenAIResponses),
    })
}

fn parse_chat_assistant_turn(json: &Value) -> Result<AssistantTurn, String> {
    let message = json
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .ok_or_else(|| "Provider returned no assistant message".to_string())?;

    let mut tool_calls = Vec::new();
    if let Some(items) = message.get("tool_calls").and_then(Value::as_array) {
        for item in items {
            tool_calls.push(ToolCall {
                id: item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("native-tool-call")
                    .to_string(),
                name: item
                    .get("function")
                    .and_then(|function| function.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                input: parse_tool_arguments(
                    message_tool_arguments(item).or_else(|| item.get("arguments")),
                ),
            });
        }
    }

    let text = if let Some(content) = message.get("content").and_then(Value::as_str) {
        content.to_string()
    } else {
        String::new()
    };

    Ok(AssistantTurn {
        text,
        tool_calls,
        usage: extract_usage(json, WireApi::OpenAIChatCompletions),
    })
}

fn message_tool_arguments(item: &Value) -> Option<&Value> {
    item.get("function")
        .and_then(|function| function.get("arguments"))
}

fn parse_tool_arguments(value: Option<&Value>) -> Value {
    match value {
        Some(Value::Object(map)) => Value::Object(map.clone()),
        Some(Value::String(text)) => {
            serde_json::from_str::<Value>(text).unwrap_or_else(|_| json!({}))
        }
        Some(other) => other.clone(),
        None => json!({}),
    }
}

fn execute_tool_call(tool_call: &ToolCall, session: &mut NativeToolSession) -> ToolExecution {
    match tool_call.name.as_str() {
        "Read" => execute_read_tool(&tool_call.input, session),
        "Grep" => execute_grep_tool(&tool_call.input, &session.cwd),
        "Glob" => execute_glob_tool(&tool_call.input, &session.cwd),
        "Bash" => execute_bash_tool(&tool_call.input, session),
        "Write" => execute_write_tool(&tool_call.input, session),
        "Edit" => execute_edit_tool(&tool_call.input, session),
        "WebSearch" => execute_web_search_tool(&tool_call.input),
        "WebFetch" => execute_web_fetch_tool(&tool_call.input),
        other => ToolExecution::Output(format!(
            "Tool {other} is not available in native local-tools mode."
        )),
    }
}

fn execute_read_tool(input: &Value, session: &mut NativeToolSession) -> ToolExecution {
    let file_path = match get_string(input, "file_path") {
        Some(value) if !value.trim().is_empty() => value,
        _ => return ToolExecution::Output("Read error: file_path is required.".to_string()),
    };

    if let Some(pages) = get_string(input, "pages") {
        if !pages.trim().is_empty() {
            return ToolExecution::FallbackToRustFullCli(
                "Read.pages is not supported natively yet".to_string(),
            );
        }
    }

    let resolved = match resolve_path_within_cwd(&session.cwd, &file_path, false) {
        Ok(path) => path,
        Err(message) => return ToolExecution::Output(format!("Read error: {message}")),
    };
    let text_file = match read_text_file_with_metadata(&resolved) {
        Ok(text_file) => text_file,
        Err(message) => {
            return ToolExecution::FallbackToRustFullCli(format!(
                "Read target {} requires Rust full CLI fallback: {message}",
                resolved.display()
            ))
        }
    };

    let content = text_file.content;
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        session.update_snapshot(&resolved, content, text_file.timestamp_ms, false);
        return ToolExecution::Output(format!("{} is empty.", session.display_path(&resolved)));
    }

    let offset = get_usize(input, "offset").unwrap_or(0);
    let start_index = offset.saturating_sub(1);
    let limit = get_usize(input, "limit")
        .unwrap_or(DEFAULT_READ_LIMIT)
        .max(1);
    let end_index = start_index.saturating_add(limit).min(lines.len());

    if start_index >= lines.len() {
        return ToolExecution::Output(format!(
            "Read error: offset {} is beyond the end of {}.",
            offset,
            session.display_path(&resolved)
        ));
    }

    let is_partial = start_index != 0 || end_index < lines.len();
    session.update_snapshot(
        &resolved,
        content.clone(),
        text_file.timestamp_ms,
        is_partial,
    );

    let mut output = String::new();
    output.push_str(&format!("{}\n", session.display_path(&resolved)));
    for (line_number, line) in lines[start_index..end_index].iter().enumerate() {
        output.push_str(&format!("{:>6}\t{}\n", start_index + line_number + 1, line));
    }
    if end_index < lines.len() {
        output.push_str(&format!(
            "\n[Read truncated: returned {} lines starting at line {} of {} total lines]",
            end_index - start_index,
            start_index + 1,
            lines.len()
        ));
    }

    ToolExecution::Output(output.trim_end().to_string())
}

fn execute_write_tool(input: &Value, session: &mut NativeToolSession) -> ToolExecution {
    let file_path = match get_string(input, "file_path") {
        Some(value) if !value.trim().is_empty() => value,
        _ => return ToolExecution::Output("Write error: file_path is required.".to_string()),
    };
    let content = match input.get("content") {
        Some(Value::String(text)) => text.to_string(),
        Some(_) => {
            return ToolExecution::Output("Write error: content must be a string.".to_string())
        }
        None => return ToolExecution::Output("Write error: content is required.".to_string()),
    };

    let (resolved, existed_before) = match resolve_path_for_write(&session.cwd, &file_path) {
        Ok(result) => result,
        Err(message) => return ToolExecution::Output(format!("Write error: {message}")),
    };

    let (encoding, stale_result) = if existed_before {
        let current = match read_text_file_with_metadata(&resolved) {
            Ok(current) => current,
            Err(message) => {
                return ToolExecution::FallbackToRustFullCli(format!(
                    "Write target {} requires Rust full CLI fallback: {message}",
                    resolved.display()
                ))
            }
        };
        let Some(snapshot) = session.snapshot(&resolved) else {
            return ToolExecution::Output(
                "Write error: File has not been read yet. Read it first before writing to it."
                    .to_string(),
            );
        };
        if snapshot.is_partial_view {
            return ToolExecution::Output(
                "Write error: File has only been partially read. Read the full file before writing to it."
                    .to_string(),
            );
        }
        (
            current.encoding,
            validate_snapshot_is_current(snapshot, &current)
                .map_err(|message| ToolExecution::Output(format!("Write error: {message}"))),
        )
    } else {
        (NativeTextEncoding::Utf8, Ok(()))
    };

    if let Err(tool_execution) = stale_result {
        return tool_execution;
    }

    if let Some(parent) = resolved.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            return ToolExecution::Output(format!(
                "Write error: failed to create parent directory {}: {error}",
                parent.display()
            ));
        }
    }
    if let Err(message) = write_text_file(&resolved, &content, encoding, NativeLineEnding::Lf) {
        return ToolExecution::Output(format!("Write error: {message}"));
    }

    match file_timestamp_ms(&resolved) {
        Ok(timestamp_ms) => {
            session.update_snapshot(
                &resolved,
                normalize_line_endings(&content),
                timestamp_ms,
                false,
            );
        }
        Err(message) => return ToolExecution::Output(format!("Write error: {message}")),
    }

    let display_path = session.display_path(&resolved);
    if existed_before {
        ToolExecution::Output(format!(
            "The file {display_path} has been updated successfully."
        ))
    } else {
        ToolExecution::Output(format!("File created successfully at: {display_path}"))
    }
}

fn execute_edit_tool(input: &Value, session: &mut NativeToolSession) -> ToolExecution {
    let file_path = match get_string(input, "file_path") {
        Some(value) if !value.trim().is_empty() => value,
        _ => return ToolExecution::Output("Edit error: file_path is required.".to_string()),
    };
    let old_string = match input.get("old_string") {
        Some(Value::String(text)) => text.to_string(),
        Some(_) => {
            return ToolExecution::Output("Edit error: old_string must be a string.".to_string())
        }
        None => return ToolExecution::Output("Edit error: old_string is required.".to_string()),
    };
    let new_string = match input.get("new_string") {
        Some(Value::String(text)) => text.to_string(),
        Some(_) => {
            return ToolExecution::Output("Edit error: new_string must be a string.".to_string())
        }
        None => return ToolExecution::Output("Edit error: new_string is required.".to_string()),
    };
    let replace_all = get_bool(input, "replace_all").unwrap_or(false);

    if old_string == new_string {
        return ToolExecution::Output(
            "Edit error: No changes to make: old_string and new_string are exactly the same."
                .to_string(),
        );
    }

    let (resolved, existed_before) = match resolve_path_for_write(&session.cwd, &file_path) {
        Ok(result) => result,
        Err(message) => return ToolExecution::Output(format!("Edit error: {message}")),
    };

    if !existed_before {
        if old_string.is_empty() {
            if let Some(parent) = resolved.parent() {
                if let Err(error) = fs::create_dir_all(parent) {
                    return ToolExecution::Output(format!(
                        "Edit error: failed to create parent directory {}: {error}",
                        parent.display()
                    ));
                }
            }
            if let Err(message) = write_text_file(
                &resolved,
                &new_string,
                NativeTextEncoding::Utf8,
                NativeLineEnding::Lf,
            ) {
                return ToolExecution::Output(format!("Edit error: {message}"));
            }
            match file_timestamp_ms(&resolved) {
                Ok(timestamp_ms) => session.update_snapshot(
                    &resolved,
                    normalize_line_endings(&new_string),
                    timestamp_ms,
                    false,
                ),
                Err(message) => return ToolExecution::Output(format!("Edit error: {message}")),
            }
            return ToolExecution::Output(format!(
                "The file {} has been updated successfully.",
                session.display_path(&resolved)
            ));
        }

        return ToolExecution::Output(
            "Edit error: File does not exist. Read it or create it with empty old_string first."
                .to_string(),
        );
    }

    if resolved.extension().and_then(|value| value.to_str()) == Some("ipynb") {
        return ToolExecution::FallbackToRustFullCli(
            "Edit on .ipynb files requires Rust full CLI fallback".to_string(),
        );
    }

    let current = match read_text_file_with_metadata(&resolved) {
        Ok(current) => current,
        Err(message) => {
            return ToolExecution::FallbackToRustFullCli(format!(
                "Edit target {} requires Rust full CLI fallback: {message}",
                resolved.display()
            ))
        }
    };

    if session.snapshot(&resolved).is_none() {
        session.update_snapshot(
            &resolved,
            current.content.clone(),
            current.timestamp_ms,
            false,
        );
    }
    let Some(snapshot) = session.snapshot(&resolved) else {
        return ToolExecution::Output(
            "Edit error: failed to initialize file snapshot before editing.".to_string(),
        );
    };
    if snapshot.is_partial_view {
        return ToolExecution::Output(
            "Edit error: File has only been partially read. Read the full file before editing it."
                .to_string(),
        );
    }
    if let Err(message) = validate_snapshot_is_current(snapshot, &current) {
        return ToolExecution::Output(format!("Edit error: {message}"));
    }

    if old_string.is_empty() {
        if !current.content.trim().is_empty() {
            return ToolExecution::Output(
                "Edit error: Cannot create new file - file already exists.".to_string(),
            );
        }

        if let Err(message) = write_text_file(
            &resolved,
            &new_string,
            current.encoding,
            current.line_endings,
        ) {
            return ToolExecution::Output(format!("Edit error: {message}"));
        }
        match file_timestamp_ms(&resolved) {
            Ok(timestamp_ms) => session.update_snapshot(
                &resolved,
                normalize_line_endings(&new_string),
                timestamp_ms,
                false,
            ),
            Err(message) => return ToolExecution::Output(format!("Edit error: {message}")),
        }
        return ToolExecution::Output(format!(
            "The file {} has been updated successfully.",
            session.display_path(&resolved)
        ));
    }

    let Some(actual_old_string) = find_actual_string(&current.content, &old_string) else {
        return ToolExecution::Output(format!(
            "Edit error: String to replace not found in file.\nString: {old_string}"
        ));
    };
    let matches = count_substring_occurrences(&current.content, &actual_old_string);
    if matches > 1 && !replace_all {
        return ToolExecution::Output(format!(
            "Edit error: Found {matches} matches of the string to replace, but replace_all is false. To replace all occurrences, set replace_all to true. To replace only one occurrence, provide more context to uniquely identify the instance.\nString: {old_string}"
        ));
    }

    let actual_new_string = preserve_quote_style(&old_string, &actual_old_string, &new_string);
    let updated = apply_edit_to_file(
        &current.content,
        &actual_old_string,
        &actual_new_string,
        replace_all,
    );
    if let Err(message) =
        write_text_file(&resolved, &updated, current.encoding, current.line_endings)
    {
        return ToolExecution::Output(format!("Edit error: {message}"));
    }

    match file_timestamp_ms(&resolved) {
        Ok(timestamp_ms) => session.update_snapshot(&resolved, updated, timestamp_ms, false),
        Err(message) => return ToolExecution::Output(format!("Edit error: {message}")),
    }

    let display_path = session.display_path(&resolved);
    if replace_all && matches > 1 {
        ToolExecution::Output(format!(
            "The file {display_path} has been updated. All occurrences were successfully replaced."
        ))
    } else {
        ToolExecution::Output(format!(
            "The file {display_path} has been updated successfully."
        ))
    }
}

fn execute_grep_tool(input: &Value, cwd: &Path) -> ToolExecution {
    let pattern = match get_string(input, "pattern") {
        Some(value) if !value.trim().is_empty() => value,
        _ => return ToolExecution::Output("Grep error: pattern is required.".to_string()),
    };
    let search_path = match get_optional_grep_search_path(input, cwd) {
        Ok(path) => path,
        Err(message) => return ToolExecution::Output(format!("Grep error: {message}")),
    };

    let output_mode = get_string(input, "output_mode").unwrap_or_else(|| {
        if search_path.is_file() {
            "content".to_string()
        } else {
            "files_with_matches".to_string()
        }
    });
    let mut command = Command::new("rg");
    command.arg("--color").arg("never");
    command.arg("--no-heading");

    match output_mode.as_str() {
        "files_with_matches" => {
            command.arg("-l");
        }
        "count" => {
            command.arg("-c");
        }
        "content" | "text" => {
            if get_bool(input, "-n").unwrap_or(true) {
                command.arg("-n");
            }
            if let Some(context) = get_usize(input, "-C").or_else(|| get_usize(input, "context")) {
                command.arg("-C").arg(context.to_string());
            } else {
                if let Some(before) = get_usize(input, "-B") {
                    command.arg("-B").arg(before.to_string());
                }
                if let Some(after) = get_usize(input, "-A") {
                    command.arg("-A").arg(after.to_string());
                }
            }
        }
        other => {
            return ToolExecution::Output(format!("Grep error: unsupported output_mode {other}."))
        }
    }

    if get_bool(input, "-i").unwrap_or(false) {
        command.arg("-i");
    }
    if get_bool(input, "multiline").unwrap_or(false) {
        command.arg("-U").arg("--multiline-dotall");
    }
    if let Some(glob) = get_string(input, "glob") {
        if !glob.trim().is_empty() {
            command.arg("--glob").arg(glob);
        }
    }
    if let Some(file_type) = get_string(input, "type") {
        let normalized_type = file_type.trim().to_ascii_lowercase();
        if !normalized_type.is_empty() && !matches!(normalized_type.as_str(), "f" | "file") {
            command.arg("--type").arg(file_type);
        }
    }

    command.arg(pattern);
    command.arg(&search_path);

    let command_output = match command.output() {
        Ok(output) => output,
        Err(error) => {
            return ToolExecution::Output(format!("Grep error: failed to execute rg: {error}"));
        }
    };
    if !command_output.status.success() && command_output.status.code() != Some(1) {
        let stderr = String::from_utf8_lossy(&command_output.stderr)
            .trim()
            .to_string();
        return ToolExecution::Output(if stderr.is_empty() {
            "Grep error: rg exited with an unexpected status.".to_string()
        } else {
            format!("Grep error: {stderr}")
        });
    }

    let stdout = String::from_utf8_lossy(&command_output.stdout).to_string();
    let lines: Vec<String> = stdout.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return ToolExecution::Output("No matches found.".to_string());
    }

    let offset = get_usize(input, "offset").unwrap_or(0);
    let limit = get_usize(input, "head_limit").unwrap_or(DEFAULT_GREP_LIMIT);
    let (sliced, truncated) = slice_lines(lines, offset, limit);
    let mut output = sliced.join("\n");
    if truncated {
        output.push_str(&format!(
            "\n[Results truncated: offset={}, limit={}]",
            offset, limit
        ));
    }
    ToolExecution::Output(output)
}

fn execute_glob_tool(input: &Value, cwd: &Path) -> ToolExecution {
    let pattern = match get_string(input, "pattern") {
        Some(value) if !value.trim().is_empty() => value,
        _ => return ToolExecution::Output("Glob error: pattern is required.".to_string()),
    };
    let search_path = match get_optional_glob_search_path(input, cwd) {
        Ok(path) => path,
        Err(message) => return ToolExecution::Output(format!("Glob error: {message}")),
    };

    let mut command = Command::new("rg");
    command.arg("--files");
    command.arg(&search_path);
    command.arg("-g").arg(pattern);

    let command_output = match command.output() {
        Ok(output) => output,
        Err(error) => {
            return ToolExecution::Output(format!(
                "Glob error: failed to execute rg --files: {error}"
            ))
        }
    };
    if !command_output.status.success() {
        let stderr = String::from_utf8_lossy(&command_output.stderr)
            .trim()
            .to_string();
        if !stderr.is_empty() {
            return ToolExecution::Output(format!("Glob error: {stderr}"));
        }
    }

    let stdout = String::from_utf8_lossy(&command_output.stdout).to_string();
    let lines: Vec<String> = stdout.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return ToolExecution::Output("No files found.".to_string());
    }

    let (sliced, truncated) = slice_lines(lines, 0, DEFAULT_GLOB_LIMIT);
    let mut output = sliced.join("\n");
    if truncated {
        output.push_str(&format!(
            "\n[Results truncated: limit={}]",
            DEFAULT_GLOB_LIMIT
        ));
    }
    ToolExecution::Output(output)
}

fn execute_bash_tool(input: &Value, session: &mut NativeToolSession) -> ToolExecution {
    let command = match get_string(input, "command") {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolExecution::Output("Bash error: command is required.".to_string()),
    };

    if get_bool(input, "run_in_background").unwrap_or(false) {
        return ToolExecution::FallbackToRustFullCli(
            "Bash.run_in_background is not supported natively".to_string(),
        );
    }
    if session.bash_policy == NativeBashPolicy::ReadOnly
        && get_bool(input, "dangerouslyDisableSandbox").unwrap_or(false)
    {
        return ToolExecution::FallbackToRustFullCli(
            "Bash.dangerouslyDisableSandbox is not supported natively".to_string(),
        );
    }

    if session.bash_policy == NativeBashPolicy::ReadOnly {
        if let Err(reason) = validate_readonly_bash_command(&command) {
            return ToolExecution::FallbackToRustFullCli(format!(
                "Bash command requires Rust full CLI fallback: {reason}"
            ));
        }
    } else if command.trim().is_empty() {
        return ToolExecution::Output("Bash error: command is required.".to_string());
    }

    let timeout_ms = get_usize(input, "timeout")
        .unwrap_or(DEFAULT_BASH_TIMEOUT_MS)
        .min(MAX_BASH_TIMEOUT_MS)
        .max(1);

    let mut child = match Command::new("bash")
        .arg("-lc")
        .arg(&command)
        .current_dir(&session.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return ToolExecution::Output(format!(
                "Bash error: failed to spawn shell command: {error}"
            ))
        }
    };

    let deadline = Instant::now() + Duration::from_millis(timeout_ms as u64);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return ToolExecution::Output(format!(
                        "Bash error: command timed out after {} ms.",
                        timeout_ms
                    ));
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(error) => {
                return ToolExecution::Output(format!(
                    "Bash error: failed while waiting for command: {error}"
                ))
            }
        }
    }

    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(error) => {
            return ToolExecution::Output(format!(
                "Bash error: failed to collect command output: {error}"
            ))
        }
    };

    if session.bash_policy == NativeBashPolicy::Unrestricted {
        session.clear_snapshots();
    }

    ToolExecution::Output(format_bash_output(&command, &output))
}

fn execute_web_search_tool(input: &Value) -> ToolExecution {
    let query = match get_string(input, "query") {
        Some(value) if value.trim().len() >= 2 => value.trim().to_string(),
        _ => return ToolExecution::Output("WebSearch error: query is required.".to_string()),
    };

    let allowed_domains = get_string_array(input, "allowed_domains");
    let blocked_domains = get_string_array(input, "blocked_domains");

    match run_native_web_search(&query, &allowed_domains, &blocked_domains) {
        Ok(result) => ToolExecution::Output(format_web_search_tool_output(&result)),
        Err(message) => ToolExecution::FallbackToRustFullCli(message),
    }
}

fn execute_web_fetch_tool(input: &Value) -> ToolExecution {
    let url = match get_string(input, "url") {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return ToolExecution::Output("WebFetch error: url is required.".to_string()),
    };
    let prompt = match get_string(input, "prompt") {
        Some(value) if !value.trim().is_empty() => value,
        _ => return ToolExecution::Output("WebFetch error: prompt is required.".to_string()),
    };

    match run_native_web_fetch(&url, &prompt) {
        Ok(result) => ToolExecution::Output(result),
        Err(WebFetchExecutionError::Output(message)) => ToolExecution::Output(message),
        Err(WebFetchExecutionError::Fallback(reason)) => {
            ToolExecution::FallbackToRustFullCli(reason)
        }
    }
}

pub fn execute_stable_web_search(
    query: &str,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Result<String, String> {
    match execute_web_search_tool(&json!({
        "query": query,
        "allowed_domains": allowed_domains,
        "blocked_domains": blocked_domains,
    })) {
        ToolExecution::Output(output) => Ok(output),
        ToolExecution::FallbackToRustFullCli(reason) => Err(reason),
    }
}

pub fn execute_stable_web_fetch(url: &str, prompt: &str) -> Result<String, String> {
    match execute_web_fetch_tool(&json!({
        "url": url,
        "prompt": prompt,
    })) {
        ToolExecution::Output(output) => Ok(output),
        ToolExecution::FallbackToRustFullCli(reason) => Err(reason),
    }
}

enum WebFetchExecutionError {
    Output(String),
    Fallback(String),
}

fn run_native_web_search(
    query: &str,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Result<NativeWebSearchOutput, String> {
    match execute_direct_web_search(query, allowed_domains, blocked_domains) {
        Ok(result) if !result.hits.is_empty() => Ok(result),
        Ok(_) | Err(_) => execute_sai_search_fallback(query, allowed_domains, blocked_domains),
    }
}

fn run_native_web_fetch(url: &str, prompt: &str) -> Result<String, WebFetchExecutionError> {
    let response = fetch_native_webfetch_content(url).map_err(WebFetchExecutionError::Fallback)?;
    match response {
        NativeWebFetchResponse::Redirect(info) => Ok(format_redirect_message(&info, prompt)),
        NativeWebFetchResponse::Content {
            content,
            content_type,
        } => {
            let processed = apply_prompt_to_native_webfetch_content(prompt, &content)
                .map_err(WebFetchExecutionError::Fallback)?;
            if processed.trim().is_empty() {
                return Err(WebFetchExecutionError::Output(
                    "WebFetch error: secondary model returned no response.".to_string(),
                ));
            }

            let mut result = processed;
            if content_type.to_ascii_lowercase().contains("text/markdown")
                && content.chars().count() > DEFAULT_WEB_FETCH_MARKDOWN_LENGTH
            {
                result.push_str("\n\n[Content truncated due to length...]");
            }
            Ok(result)
        }
    }
}

fn execute_direct_web_search(
    query: &str,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Result<NativeWebSearchOutput, String> {
    let response = fetch_url_with_curl(
        &build_search_url(query),
        "text/html, text/plain;q=0.9, */*;q=0.8",
        SEARCH_USER_AGENT,
        DEFAULT_WEB_SEARCH_TIMEOUT_MS,
        Some(10),
    )?;

    let html = String::from_utf8_lossy(&response.body).to_string();
    let mut hits = extract_search_hits(&html);
    if hits.is_empty() {
        hits = extract_search_hits_from_generic_links(&html);
    }

    let normalized_hits = normalize_hits(query, hits, allowed_domains, blocked_domains);
    let fetched_pages = fetch_top_page_excerpts(query, &normalized_hits);

    Ok(NativeWebSearchOutput {
        query: query.to_string(),
        hits: normalized_hits,
        fetched_pages,
    })
}

fn execute_sai_search_fallback(
    query: &str,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Result<NativeWebSearchOutput, String> {
    let mut last_error: Option<String> = None;

    for base_url in get_sai_search_base_urls() {
        match execute_sai_search_http(&base_url, query, allowed_domains, blocked_domains) {
            Ok(result) if !result.hits.is_empty() => return Ok(result),
            Ok(_) => {
                last_error = Some(format!(
                    "Native WebSearch HTTP fallback at {base_url} returned no hits"
                ));
            }
            Err(error) => {
                last_error = Some(error);
            }
        }
    }

    if let Some(target) = get_sai_search_ssh_target() {
        return execute_sai_search_over_ssh(&target, query, allowed_domains, blocked_domains);
    }

    Err(last_error.unwrap_or_else(|| {
        "Native WebSearch could not obtain results and no native sai-search fallback is configured"
            .to_string()
    }))
}

fn execute_sai_search_http(
    base_url: &str,
    query: &str,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Result<NativeWebSearchOutput, String> {
    let endpoint = join_base_url_with_path(base_url, "/search");
    let payload = json!({
        "query": query,
        "limit": DEFAULT_MAX_SEARCH_RESULTS,
        "include_content": false,
        "progress": false,
        "mode": "single",
    });
    let response = run_simple_json_post(
        &endpoint,
        &payload,
        SEARCH_USER_AGENT,
        DEFAULT_WEB_SEARCH_SAI_TIMEOUT_MS,
    )?;
    let raw_hits = extract_sai_search_hits(&response)?;
    let hits = normalize_hits(query, raw_hits, allowed_domains, blocked_domains);
    let fetched_pages = fetch_top_page_excerpts(query, &hits);
    Ok(NativeWebSearchOutput {
        query: query.to_string(),
        hits,
        fetched_pages,
    })
}

fn execute_sai_search_over_ssh(
    target: &str,
    query: &str,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Result<NativeWebSearchOutput, String> {
    let remote_url = env::var("SAICODE_SAI_SEARCH_SSH_REMOTE_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_SAI_SEARCH_SSH_REMOTE_URL.to_string());
    let payload = json!({
        "query": query,
        "limit": DEFAULT_MAX_SEARCH_RESULTS,
        "include_content": false,
        "progress": false,
        "mode": "single",
    })
    .to_string();
    let remote_command = format!(
        "curl -sS -X POST '{}' -H 'content-type: application/json' --data-binary @-",
        remote_url.replace('\'', "%27")
    );

    let mut child = Command::new("ssh")
        .arg(target)
        .arg(remote_command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Native WebSearch SSH fallback failed to start: {error}"))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(payload.as_bytes()).map_err(|error| {
            format!("Native WebSearch SSH fallback failed to write stdin: {error}")
        })?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("Native WebSearch SSH fallback failed: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Native WebSearch SSH fallback failed".to_string()
        } else {
            format!("Native WebSearch SSH fallback failed: {stderr}")
        });
    }

    let stdout = String::from_utf8(output.stdout).map_err(|error| {
        format!("Native WebSearch SSH fallback returned invalid UTF-8: {error}")
    })?;
    let response: Value = serde_json::from_str(&stdout)
        .map_err(|error| format!("Native WebSearch SSH fallback returned invalid JSON: {error}"))?;
    let raw_hits = extract_sai_search_hits(&response)?;
    let hits = normalize_hits(query, raw_hits, allowed_domains, blocked_domains);
    let fetched_pages = fetch_top_page_excerpts(query, &hits);
    Ok(NativeWebSearchOutput {
        query: query.to_string(),
        hits,
        fetched_pages,
    })
}

fn fetch_native_webfetch_content(url: &str) -> Result<NativeWebFetchResponse, String> {
    let parsed = parse_http_url(url)
        .ok_or_else(|| "Native WebFetch only supports valid http/https URLs".to_string())?;
    if url.len() > 2_000 {
        return Err("Native WebFetch only supports URLs up to 2000 characters".to_string());
    }
    if parsed.host.split('.').count() < 2 {
        return Err("Native WebFetch requires a publicly resolvable hostname".to_string());
    }

    let mut current_url = if parsed.scheme == "http" && !is_local_host(&parsed.host) {
        parsed.with_scheme("https").to_string()
    } else {
        parsed.to_string()
    };

    for _ in 0..=DEFAULT_MAX_REDIRECTS {
        let response = fetch_url_with_curl(
            &current_url,
            "text/markdown, text/html, text/plain;q=0.9, */*;q=0.8",
            WEB_FETCH_USER_AGENT,
            DEFAULT_WEB_FETCH_TIMEOUT_MS,
            None,
        )?;

        if is_redirect_status(response.status_code) {
            let location = get_header_value(&response.headers, "location").ok_or_else(|| {
                "Native WebFetch redirect response missing Location header".to_string()
            })?;
            let redirect_url = resolve_redirect_url(&current_url, location)
                .ok_or_else(|| "Native WebFetch could not resolve redirect URL".to_string())?;
            if is_permitted_redirect(&current_url, &redirect_url) {
                current_url = redirect_url;
                continue;
            }
            return Ok(NativeWebFetchResponse::Redirect(RedirectInfo {
                original_url: current_url,
                redirect_url,
                status_code: response.status_code,
            }));
        }

        if !is_textual_content_type(&response.content_type) || response.body.contains(&0) {
            return Err(
                "Native WebFetch does not support binary or non-text content yet".to_string(),
            );
        }

        if response.body.len() > MAX_HTTP_CONTENT_LENGTH {
            return Err("Native WebFetch content exceeded the native size limit".to_string());
        }

        let raw_text = String::from_utf8_lossy(&response.body).to_string();
        let content = if response
            .content_type
            .to_ascii_lowercase()
            .contains("text/html")
        {
            html_to_text(&extract_primary_html_region(&raw_text))
        } else {
            collapse_whitespace(&raw_text)
        };

        let content = truncate_chars(&content, DEFAULT_WEB_FETCH_MARKDOWN_LENGTH);
        return Ok(NativeWebFetchResponse::Content {
            content,
            content_type: response.content_type,
        });
    }

    Err(format!(
        "Native WebFetch exceeded redirect limit ({DEFAULT_MAX_REDIRECTS})"
    ))
}

fn apply_prompt_to_native_webfetch_content(prompt: &str, content: &str) -> Result<String, String> {
    let small_fast_model = env::var("SAICODE_SMALL_FAST_MODEL")
        .ok()
        .or_else(|| env::var("SAICODE_DEFAULT_HAIKU_MODEL").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SMALL_FAST_MODEL_ID.to_string());
    let resolved_model = resolve_model(Some(&small_fast_model));
    let provider = get_provider_config(&resolved_model)?;
    let user_prompt = build_native_webfetch_secondary_prompt(content, prompt);
    let messages = vec![ConversationMessage::UserText(user_prompt)];
    let body = build_request_body(
        &provider,
        &resolved_model,
        None,
        &messages,
        &[],
        NativeBashPolicy::ReadOnly,
    );
    let turn = if provider.api == WireApi::OpenAIChatCompletions {
        parse_streamed_assistant_turn(execute_provider_chat_completions_stream(
            &provider, &body, false,
        )?)
    } else {
        let response = execute_provider_json_request(&provider, &body)?;
        parse_assistant_turn(&response, provider.api)?
    };
    Ok(turn.text.trim().to_string())
}

fn format_redirect_message(info: &RedirectInfo, prompt: &str) -> String {
    let status_text = match info.status_code {
        301 => "Moved Permanently",
        308 => "Permanent Redirect",
        307 => "Temporary Redirect",
        _ => "Found",
    };

    format!(
        "REDIRECT DETECTED: The URL redirects to a different host.\n\nOriginal URL: {}\nRedirect URL: {}\nStatus: {} {}\n\nTo complete your request, I need to fetch content from the redirected URL. Please use WebFetch again with these parameters:\n- url: \"{}\"\n- prompt: \"{}\"",
        info.original_url,
        info.redirect_url,
        info.status_code,
        status_text,
        info.redirect_url,
        prompt.replace('"', "\\\""),
    )
}

fn format_web_search_tool_output(output: &NativeWebSearchOutput) -> String {
    let mut rendered = format!("Web search results for query: \"{}\"\n\n", output.query);
    if let Some(top_domain) = output
        .hits
        .first()
        .and_then(|hit| parse_http_url(&hit.url).map(|parsed| parsed.host))
    {
        rendered.push_str(&format!("Top result domain: {top_domain}\n\n"));
    }
    rendered.push_str(&build_search_summary(&output.query, &output.hits));
    rendered.push_str("\n\n");

    if let Some(summary) = build_fetched_pages_summary(&output.query, &output.fetched_pages) {
        rendered.push_str(&summary);
        rendered.push_str("\n\n");
    }

    let links_payload: Vec<Value> = output
        .hits
        .iter()
        .map(|hit| json!({ "title": hit.title, "url": hit.url }))
        .collect();
    let links_json = serde_json::to_string(&links_payload).unwrap_or_else(|_| "[]".to_string());
    rendered.push_str(&format!("Links: {links_json}\n\n"));
    rendered.push_str(
        "REMINDER: You MUST include the sources above in your response to the user using markdown hyperlinks.",
    );
    rendered.trim().to_string()
}

fn format_bash_output(command: &str, output: &std::process::Output) -> String {
    let exit_code = output
        .status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "signal".to_string());
    let (stdout, stdout_truncated) =
        truncate_output_bytes(&output.stdout, DEFAULT_BASH_OUTPUT_LIMIT);
    let (stderr, stderr_truncated) =
        truncate_output_bytes(&output.stderr, DEFAULT_BASH_OUTPUT_LIMIT / 2);

    let stdout = stdout.trim_end().to_string();
    let stderr = stderr.trim_end().to_string();

    let mut out = String::new();
    out.push_str(&format!("command: {command}\n"));
    out.push_str(&format!("exit_code: {exit_code}\n"));

    if !stdout.is_empty() {
        out.push_str("stdout:\n");
        out.push_str(&stdout);
        out.push('\n');
        if stdout_truncated {
            out.push_str("[stdout truncated]\n");
        }
    } else if output.status.success() {
        out.push_str("stdout:\n(no output)\n");
    }

    if !stderr.is_empty() {
        out.push_str("stderr:\n");
        out.push_str(&stderr);
        out.push('\n');
        if stderr_truncated {
            out.push_str("[stderr truncated]\n");
        }
    }

    out.trim_end().to_string()
}

fn truncate_output_bytes(bytes: &[u8], limit: usize) -> (String, bool) {
    if bytes.len() <= limit {
        return (String::from_utf8_lossy(bytes).to_string(), false);
    }

    (String::from_utf8_lossy(&bytes[..limit]).to_string(), true)
}

fn validate_readonly_bash_command(command: &str) -> Result<(), String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err("empty command".to_string());
    }
    if trimmed.len() > 4_000 {
        return Err("command is too long for native readonly Bash".to_string());
    }

    for token in ["\n", "\r", "$(", "`", "<<<", "<<", "<", ">", "|&"] {
        if trimmed.contains(token) {
            return Err(format!("shell syntax `{token}` is not supported natively"));
        }
    }
    if trimmed.contains('$') {
        return Err("shell variable expansion is not supported natively".to_string());
    }
    if trimmed.contains('{') || trimmed.contains('}') {
        return Err("brace-based shell syntax is not supported natively".to_string());
    }
    if trimmed.contains('*')
        || trimmed.contains('?')
        || trimmed.contains('[')
        || trimmed.contains(']')
    {
        return Err("shell glob expansion is not supported natively".to_string());
    }

    let mut chars = trimmed.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '&' {
            if chars.peek() == Some(&'&') {
                chars.next();
                continue;
            }
            return Err("background operators are not supported natively".to_string());
        }
    }

    let segments = split_shell_segments(trimmed)?;
    for segment in segments {
        validate_readonly_bash_segment(&segment)?;
    }

    Ok(())
}

fn split_shell_segments(command: &str) -> Result<Vec<String>, String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => {
                escaped = true;
                current.push(ch);
            }
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(ch);
            }
            '|' if !in_single && !in_double => {
                if chars.peek() == Some(&'|') {
                    chars.next();
                }
                push_shell_segment(&mut segments, &mut current)?;
            }
            ';' if !in_single && !in_double => {
                push_shell_segment(&mut segments, &mut current)?;
            }
            '&' if !in_single && !in_double => {
                if chars.peek() == Some(&'&') {
                    chars.next();
                    push_shell_segment(&mut segments, &mut current)?;
                } else {
                    return Err("background operators are not supported natively".to_string());
                }
            }
            _ => current.push(ch),
        }
    }

    if escaped || in_single || in_double {
        return Err("command has unterminated shell quoting".to_string());
    }

    push_shell_segment(&mut segments, &mut current)?;
    Ok(segments)
}

fn push_shell_segment(segments: &mut Vec<String>, current: &mut String) -> Result<(), String> {
    let trimmed = current.trim();
    if trimmed.is_empty() {
        return Err("empty shell segment is not supported natively".to_string());
    }
    segments.push(trimmed.to_string());
    current.clear();
    Ok(())
}

fn tokenize_shell_words(command: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' if !in_single => {
                escaped = true;
            }
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            ch if ch.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if escaped || in_single || in_double {
        return Err("command has unterminated shell quoting".to_string());
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

fn validate_readonly_bash_segment(segment: &str) -> Result<(), String> {
    let tokens = tokenize_shell_words(segment)?;
    if tokens.is_empty() {
        return Err("empty Bash command is not supported natively".to_string());
    }
    if tokens[0].contains('=') {
        return Err("environment-prefixed shell commands are not supported natively".to_string());
    }

    match tokens[0].as_str() {
        "pwd" | "ls" | "cat" | "head" | "wc" | "stat" | "file" | "which" | "uname" | "printenv"
        | "basename" | "dirname" | "rg" | "grep" | "fd" | "fdfind" | "tree" | "du" | "echo"
        | "printf" | "true" | "false" | ":" => Ok(()),
        "tail" => validate_tail_command(&tokens),
        "date" => validate_date_command(&tokens),
        "command" => validate_command_builtin(&tokens),
        "type" => validate_type_builtin(&tokens),
        "find" => validate_find_command(&tokens),
        "git" => validate_git_command(&tokens),
        other => Err(format!(
            "command `{other}` is outside the native readonly Bash subset"
        )),
    }
}

fn validate_tail_command(tokens: &[String]) -> Result<(), String> {
    if tokens.iter().skip(1).any(|token| {
        matches!(
            token.as_str(),
            "-f" | "-F" | "--follow" | "--pid" | "--retry" | "-s" | "--sleep-interval"
        ) || token.starts_with("--follow=")
    }) {
        return Err("tail follow mode is not supported natively".to_string());
    }
    Ok(())
}

fn validate_date_command(tokens: &[String]) -> Result<(), String> {
    for token in tokens.iter().skip(1) {
        if token.starts_with('-') || token.starts_with('+') {
            continue;
        }
        return Err(
            "date positional arguments that could set system time are not supported natively"
                .to_string(),
        );
    }
    Ok(())
}

fn validate_command_builtin(tokens: &[String]) -> Result<(), String> {
    if tokens.len() < 3 {
        return Err("command must be used like `command -v <name>` in native mode".to_string());
    }
    match tokens[1].as_str() {
        "-v" | "-V" => Ok(()),
        _ => Err("native mode only supports `command -v` or `command -V`".to_string()),
    }
}

fn validate_type_builtin(tokens: &[String]) -> Result<(), String> {
    if tokens.len() < 2 {
        return Err("type requires at least one target".to_string());
    }
    Ok(())
}

fn validate_find_command(tokens: &[String]) -> Result<(), String> {
    if tokens.iter().skip(1).any(|token| {
        matches!(
            token.as_str(),
            "-exec"
                | "-execdir"
                | "-ok"
                | "-okdir"
                | "-delete"
                | "-fprint"
                | "-fprint0"
                | "-fprintf"
                | "-fls"
        )
    }) {
        return Err("find write/exec actions are not supported natively".to_string());
    }
    Ok(())
}

fn validate_git_command(tokens: &[String]) -> Result<(), String> {
    if tokens.len() < 2 {
        return Err("git subcommand is required".to_string());
    }

    let mut index = 1;
    while index < tokens.len() && tokens[index] == "--no-pager" {
        index += 1;
    }
    if index >= tokens.len() {
        return Err("git subcommand is required".to_string());
    }
    if tokens[index].starts_with('-') {
        return Err("git global flags beyond --no-pager are not supported natively".to_string());
    }

    let subcommand = tokens[index].as_str();
    match subcommand {
        "status" | "diff" | "log" | "show" | "rev-parse" | "ls-files" | "grep" | "blame"
        | "show-ref" | "describe" => {}
        other => {
            return Err(format!(
                "git subcommand `{other}` is outside the native readonly Bash subset"
            ))
        }
    }

    for token in tokens.iter().skip(1) {
        if token == "-c"
            || token.starts_with("-c")
            || token == "--config-env"
            || token.starts_with("--config-env=")
            || token == "--exec-path"
            || token.starts_with("--exec-path=")
            || token == "--output"
            || token.starts_with("--output=")
            || token == "--paginate"
        {
            return Err("git command includes unsupported or risky flags".to_string());
        }
    }

    Ok(())
}

fn slice_lines(lines: Vec<String>, offset: usize, limit: usize) -> (Vec<String>, bool) {
    if limit == 0 {
        return (lines.into_iter().skip(offset).collect(), false);
    }

    let skipped = if offset >= lines.len() {
        Vec::new()
    } else {
        lines[offset..].to_vec()
    };
    let truncated = skipped.len() > limit;
    let sliced = skipped.into_iter().take(limit).collect();
    (sliced, truncated)
}

fn get_optional_grep_search_path(input: &Value, cwd: &Path) -> Result<PathBuf, String> {
    match get_string(input, "path") {
        Some(path) if !path.trim().is_empty() => resolve_existing_path_within_cwd(cwd, &path),
        _ => Ok(cwd.to_path_buf()),
    }
}

fn get_optional_glob_search_path(input: &Value, cwd: &Path) -> Result<PathBuf, String> {
    match get_string(input, "path") {
        Some(path) if !path.trim().is_empty() => resolve_path_within_cwd(cwd, &path, true),
        _ => Ok(cwd.to_path_buf()),
    }
}

fn canonicalize_cwd_root(cwd: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(cwd)
        .map_err(|error| format!("Failed to resolve current working directory {}: {error}", cwd.display()))
}

fn ensure_within_cwd(cwd: &Path, canonical: &Path) -> Result<(), String> {
    let canonical_cwd = canonicalize_cwd_root(cwd)?;
    if canonical.starts_with(&canonical_cwd) {
        Ok(())
    } else {
        Err(format!(
            "{} is outside the current working directory {}; native local-tools mode only supports cwd-contained paths",
            canonical.display(),
            cwd.display()
        ))
    }
}

fn resolve_path_within_cwd(
    cwd: &Path,
    raw_path: &str,
    expect_directory: bool,
) -> Result<PathBuf, String> {
    let candidate = if Path::new(raw_path).is_absolute() {
        PathBuf::from(raw_path)
    } else {
        cwd.join(raw_path)
    };

    let metadata = fs::metadata(&candidate).map_err(|error| {
        format!(
            "{} does not exist or is not accessible: {error}",
            candidate.display()
        )
    })?;
    if expect_directory && !metadata.is_dir() {
        return Err(format!("{} is not a directory", candidate.display()));
    }
    if !expect_directory && !metadata.is_file() {
        return Err(format!("{} is not a file", candidate.display()));
    }

    let canonical = fs::canonicalize(&candidate)
        .map_err(|error| format!("Failed to resolve {}: {error}", candidate.display()))?;
    ensure_within_cwd(cwd, &canonical)?;

    Ok(canonical)
}

fn resolve_existing_path_within_cwd(cwd: &Path, raw_path: &str) -> Result<PathBuf, String> {
    let candidate = if Path::new(raw_path).is_absolute() {
        PathBuf::from(raw_path)
    } else {
        cwd.join(raw_path)
    };

    fs::metadata(&candidate).map_err(|error| {
        format!(
            "{} does not exist or is not accessible: {error}",
            candidate.display()
        )
    })?;

    let canonical = fs::canonicalize(&candidate)
        .map_err(|error| format!("Failed to resolve {}: {error}", candidate.display()))?;
    ensure_within_cwd(cwd, &canonical)?;

    Ok(canonical)
}

fn resolve_path_for_write(cwd: &Path, raw_path: &str) -> Result<(PathBuf, bool), String> {
    let candidate = normalize_candidate_path(cwd, raw_path);
    let (existing_ancestor, missing_tail) = find_existing_ancestor(&candidate)?;

    let ancestor_metadata = fs::metadata(&existing_ancestor).map_err(|error| {
        format!(
            "{} does not exist or is not accessible: {error}",
            existing_ancestor.display()
        )
    })?;
    if missing_tail.as_os_str().is_empty() && ancestor_metadata.is_file() {
        let canonical = fs::canonicalize(&existing_ancestor).map_err(|error| {
            format!("Failed to resolve {}: {error}", existing_ancestor.display())
        })?;
        ensure_within_cwd(cwd, &canonical)?;
        return Ok((canonical, true));
    }

    if !ancestor_metadata.is_dir() {
        return Err(format!(
            "Parent path {} is not a directory",
            existing_ancestor.display()
        ));
    }

    let canonical_ancestor = fs::canonicalize(&existing_ancestor)
        .map_err(|error| format!("Failed to resolve {}: {error}", existing_ancestor.display()))?;
    ensure_within_cwd(cwd, &canonical_ancestor)?;

    let mut resolved = canonical_ancestor.clone();
    if !missing_tail.as_os_str().is_empty() {
        resolved = canonical_ancestor.join(&missing_tail);
    }

    match fs::metadata(&resolved) {
        Ok(metadata) => {
            if metadata.is_dir() {
                return Err(format!("{} is a directory", resolved.display()));
            }
            let canonical = fs::canonicalize(&resolved)
                .map_err(|error| format!("Failed to resolve {}: {error}", resolved.display()))?;
            ensure_within_cwd(cwd, &canonical)?;
            Ok((canonical, true))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok((resolved, false)),
        Err(error) => Err(format!(
            "{} does not exist or is not accessible: {error}",
            resolved.display()
        )),
    }
}

fn normalize_candidate_path(cwd: &Path, raw_path: &str) -> PathBuf {
    let candidate = if Path::new(raw_path).is_absolute() {
        PathBuf::from(raw_path)
    } else {
        cwd.join(raw_path)
    };

    let mut normalized = if candidate.is_absolute() {
        PathBuf::from(std::path::MAIN_SEPARATOR.to_string())
    } else {
        PathBuf::new()
    };

    for component in candidate.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => {
                if normalized.as_os_str().is_empty() {
                    normalized.push(std::path::MAIN_SEPARATOR.to_string());
                }
            }
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }

    normalized
}

fn find_existing_ancestor(candidate: &Path) -> Result<(PathBuf, PathBuf), String> {
    let mut current = candidate.to_path_buf();
    let mut missing_components: Vec<std::ffi::OsString> = Vec::new();

    loop {
        match fs::metadata(&current) {
            Ok(_) => {
                let mut tail = PathBuf::new();
                for component in missing_components.iter().rev() {
                    tail.push(component);
                }
                return Ok((current, tail));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let Some(name) = current.file_name() else {
                    return Err(format!(
                        "No existing parent directory found for {}",
                        candidate.display()
                    ));
                };
                missing_components.push(name.to_os_string());
                let Some(parent) = current.parent() else {
                    return Err(format!(
                        "No existing parent directory found for {}",
                        candidate.display()
                    ));
                };
                current = parent.to_path_buf();
            }
            Err(error) => {
                return Err(format!(
                    "{} does not exist or is not accessible: {error}",
                    current.display()
                ))
            }
        }
    }
}

fn read_text_file_with_metadata(path: &Path) -> Result<NativeTextFile, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("failed to stat {}: {error}", path.display()))?;
    if metadata.is_dir() {
        return Err("path is a directory".to_string());
    }
    if metadata.len() > MAX_EDIT_FILE_SIZE {
        return Err(format!(
            "file exceeds native text-edit size limit ({} bytes)",
            MAX_EDIT_FILE_SIZE
        ));
    }

    let raw =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if raw.is_empty() {
        return Ok(NativeTextFile {
            content: String::new(),
            encoding: NativeTextEncoding::Utf8,
            line_endings: NativeLineEnding::Lf,
            timestamp_ms: metadata_mtime_ms(&metadata)?,
        });
    }

    let (content, encoding) = if raw.len() >= 2 && raw[0] == 0xff && raw[1] == 0xfe {
        (decode_utf16le(&raw)?, NativeTextEncoding::Utf16Le)
    } else {
        if raw.contains(&0) {
            return Err("file appears to be binary or otherwise unsupported".to_string());
        }
        (
            String::from_utf8(raw).map_err(|error| format!("file is not valid UTF-8: {error}"))?,
            NativeTextEncoding::Utf8,
        )
    };

    if content.contains('\r') && !content.contains("\r\n") {
        return Err("standalone CR line endings are not supported natively".to_string());
    }

    Ok(NativeTextFile {
        line_endings: detect_line_endings_for_string(&content),
        content: normalize_line_endings(&content),
        encoding,
        timestamp_ms: metadata_mtime_ms(&metadata)?,
    })
}

fn decode_utf16le(raw: &[u8]) -> Result<String, String> {
    if raw.len() % 2 != 0 {
        return Err("UTF-16LE file has an odd number of bytes".to_string());
    }
    let units: Vec<u16> = raw
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    String::from_utf16(&units).map_err(|error| format!("failed to decode UTF-16LE: {error}"))
}

fn metadata_mtime_ms(metadata: &fs::Metadata) -> Result<u64, String> {
    metadata
        .modified()
        .map_err(|error| format!("failed to read file modification time: {error}"))?
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("file modification time predates epoch: {error}"))
        .map(|duration| duration.as_millis() as u64)
}

fn detect_line_endings_for_string(content: &str) -> NativeLineEnding {
    let mut crlf_count = 0;
    let mut lf_count = 0;

    let mut chars = content.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' && chars.peek() == Some(&'\n') {
            crlf_count += 1;
            chars.next();
            continue;
        }
        if ch == '\n' {
            lf_count += 1;
        }
    }

    if crlf_count > lf_count {
        NativeLineEnding::Crlf
    } else {
        NativeLineEnding::Lf
    }
}

fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n")
}

fn validate_snapshot_is_current(
    snapshot: &NativeReadSnapshot,
    current: &NativeTextFile,
) -> Result<(), String> {
    if current.timestamp_ms > snapshot.timestamp_ms && current.content != snapshot.content {
        return Err(
            "File has been modified since read, either by the user or by a linter. Read it again before attempting to write it."
                .to_string(),
        );
    }
    Ok(())
}

fn file_timestamp_ms(path: &Path) -> Result<u64, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("failed to stat {}: {error}", path.display()))?;
    metadata_mtime_ms(&metadata)
}

fn write_text_file(
    path: &Path,
    content: &str,
    encoding: NativeTextEncoding,
    line_endings: NativeLineEnding,
) -> Result<(), String> {
    let normalized = match line_endings {
        NativeLineEnding::Lf => content.to_string(),
        NativeLineEnding::Crlf => normalize_line_endings(content).replace('\n', "\r\n"),
    };

    let bytes = match encoding {
        NativeTextEncoding::Utf8 => normalized.into_bytes(),
        NativeTextEncoding::Utf16Le => encode_utf16le(&normalized),
    };

    fs::write(path, bytes).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn encode_utf16le(content: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(content.len() * 2);
    for unit in content.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes
}

fn canonical_cwd() -> Result<PathBuf, String> {
    let cwd =
        env::current_dir().map_err(|error| format!("Failed to read current directory: {error}"))?;
    fs::canonicalize(&cwd).map_err(|error| {
        format!(
            "Failed to resolve current directory {}: {error}",
            cwd.display()
        )
    })
}

fn relativize_display_path(cwd: &Path, path: &Path) -> String {
    path.strip_prefix(cwd)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn get_string(input: &Value, key: &str) -> Option<String> {
    input.get(key).and_then(|value| match value {
        Value::String(text) => Some(text.to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        _ => None,
    })
}

fn get_usize(input: &Value, key: &str) -> Option<usize> {
    input.get(key).and_then(|value| match value {
        Value::Number(number) => number.as_u64().map(|value| value as usize),
        Value::String(text) => text.parse::<usize>().ok(),
        _ => None,
    })
}

fn get_bool(input: &Value, key: &str) -> Option<bool> {
    input.get(key).and_then(|value| match value {
        Value::Bool(boolean) => Some(*boolean),
        Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        },
        _ => None,
    })
}

fn normalize_quotes(input: &str) -> String {
    input
        .replace('‘', "'")
        .replace('’', "'")
        .replace('“', "\"")
        .replace('”', "\"")
}

fn find_actual_string(file_content: &str, search_string: &str) -> Option<String> {
    if file_content.contains(search_string) {
        return Some(search_string.to_string());
    }

    let normalized_search: Vec<char> = normalize_quotes(search_string).chars().collect();
    let normalized_file: Vec<char> = normalize_quotes(file_content).chars().collect();
    let original_chars: Vec<char> = file_content.chars().collect();
    let search_len = normalized_search.len();
    if search_len == 0 || search_len > normalized_file.len() {
        return None;
    }

    for start in 0..=normalized_file.len() - search_len {
        if normalized_file[start..start + search_len] == normalized_search[..] {
            return Some(original_chars[start..start + search_len].iter().collect());
        }
    }

    None
}

fn preserve_quote_style(old_string: &str, actual_old_string: &str, new_string: &str) -> String {
    if old_string == actual_old_string {
        return new_string.to_string();
    }

    let has_double_quotes = actual_old_string.contains('“') || actual_old_string.contains('”');
    let has_single_quotes = actual_old_string.contains('‘') || actual_old_string.contains('’');

    let mut result = new_string.to_string();
    if has_double_quotes {
        result = apply_curly_double_quotes(&result);
    }
    if has_single_quotes {
        result = apply_curly_single_quotes(&result);
    }
    result
}

fn is_opening_context(chars: &[char], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    matches!(
        chars[index - 1],
        ' ' | '\t' | '\n' | '\r' | '(' | '[' | '{' | '\u{2014}' | '\u{2013}'
    )
}

fn apply_curly_double_quotes(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut result = String::with_capacity(input.len());
    for (index, ch) in chars.iter().enumerate() {
        if *ch == '"' {
            result.push(if is_opening_context(&chars, index) {
                '“'
            } else {
                '”'
            });
        } else {
            result.push(*ch);
        }
    }
    result
}

fn apply_curly_single_quotes(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut result = String::with_capacity(input.len());
    for (index, ch) in chars.iter().enumerate() {
        if *ch != '\'' {
            result.push(*ch);
            continue;
        }

        let prev = index
            .checked_sub(1)
            .and_then(|position| chars.get(position));
        let next = chars.get(index + 1);
        let prev_is_letter = prev.map(|value| value.is_alphabetic()).unwrap_or(false);
        let next_is_letter = next.map(|value| value.is_alphabetic()).unwrap_or(false);

        if prev_is_letter && next_is_letter {
            result.push('’');
        } else {
            result.push(if is_opening_context(&chars, index) {
                '‘'
            } else {
                '’'
            });
        }
    }
    result
}

fn apply_edit_to_file(
    original_content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> String {
    let replace_with = |content: &str, search: &str, replace: &str| {
        if replace_all {
            content.replace(search, replace)
        } else {
            content.replacen(search, replace, 1)
        }
    };

    if !new_string.is_empty() {
        return replace_with(original_content, old_string, new_string);
    }

    let strip_trailing_newline =
        !old_string.ends_with('\n') && original_content.contains(&(old_string.to_string() + "\n"));
    if strip_trailing_newline {
        replace_with(
            original_content,
            &(old_string.to_string() + "\n"),
            new_string,
        )
    } else {
        replace_with(original_content, old_string, new_string)
    }
}

fn count_substring_occurrences(content: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    content.match_indices(needle).count()
}

fn is_env_truthy(value: Option<&str>) -> bool {
    matches!(
        value.map(|item| item.trim().to_ascii_lowercase()),
        Some(ref item) if matches!(item.as_str(), "1" | "true" | "yes" | "on")
    )
}

fn get_string_array(input: &Value, key: &str) -> Vec<String> {
    input
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match item {
                    Value::String(text) => Some(text.trim().to_string()),
                    Value::Number(number) => Some(number.to_string()),
                    Value::Bool(boolean) => Some(boolean.to_string()),
                    _ => None,
                })
                .filter(|text| !text.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

impl SimpleUrl {
    fn to_string(&self) -> String {
        let mut out = format!("{}://{}", self.scheme, self.host);
        if let Some(port) = &self.port {
            out.push(':');
            out.push_str(port);
        }
        out.push_str(&self.path_and_query);
        out
    }

    fn origin(&self) -> String {
        let mut out = format!("{}://{}", self.scheme, self.host);
        if let Some(port) = &self.port {
            out.push(':');
            out.push_str(port);
        }
        out
    }

    fn with_scheme(&self, scheme: &str) -> Self {
        Self {
            scheme: scheme.to_string(),
            host: self.host.clone(),
            port: self.port.clone(),
            path_and_query: self.path_and_query.clone(),
        }
    }

    fn path_without_query(&self) -> &str {
        self.path_and_query
            .split_once('?')
            .map(|(path, _)| path)
            .unwrap_or(self.path_and_query.as_str())
    }

    fn directory_path(&self) -> String {
        let path = self.path_without_query();
        if path.is_empty() || path == "/" {
            return "/".to_string();
        }
        if path.ends_with('/') {
            return path.to_string();
        }
        path.rsplit_once('/')
            .map(|(prefix, _)| {
                if prefix.is_empty() {
                    "/".to_string()
                } else {
                    format!("{prefix}/")
                }
            })
            .unwrap_or_else(|| "/".to_string())
    }
}

fn parse_http_url(raw: &str) -> Option<SimpleUrl> {
    let trimmed = raw.trim();
    let (scheme, rest) = if let Some(rest) = trimmed.strip_prefix("https://") {
        ("https", rest)
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        ("http", rest)
    } else {
        return None;
    };

    let split_index = rest.find(|ch| ch == '/' || ch == '?').unwrap_or(rest.len());
    let host_port = &rest[..split_index];
    if host_port.is_empty() || host_port.contains('@') || host_port.starts_with('[') {
        return None;
    }

    let suffix = &rest[split_index..];
    let path_and_query = if suffix.is_empty() {
        "/".to_string()
    } else if suffix.starts_with('?') {
        format!("/{suffix}")
    } else {
        suffix.to_string()
    };
    let path_and_query = path_and_query
        .split_once('#')
        .map(|(path, _)| path.to_string())
        .unwrap_or(path_and_query);

    let (host, port) = if let Some((host, port)) = host_port.rsplit_once(':') {
        if !host.contains(':') && !port.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()) {
            (host.to_ascii_lowercase(), Some(port.to_string()))
        } else {
            (host_port.to_ascii_lowercase(), None)
        }
    } else {
        (host_port.to_ascii_lowercase(), None)
    };

    if host.is_empty() {
        return None;
    }

    Some(SimpleUrl {
        scheme: scheme.to_string(),
        host,
        port,
        path_and_query,
    })
}

fn build_search_url(query: &str) -> String {
    let base = env::var("SAICODE_WEB_SEARCH_BASE_URL")
        .ok()
        .or_else(|| env::var("CLAWD_WEB_SEARCH_BASE_URL").ok())
        .or_else(|| env::var("CLAW_WEB_SEARCH_BASE_URL").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SEARCH_BASE_URL.to_string());
    let separator = if base.contains('?') {
        if base.ends_with('?') || base.ends_with('&') {
            ""
        } else {
            "&"
        }
    } else {
        "?"
    };
    format!("{base}{separator}q={}", url_encode_component(query))
}

fn get_sai_search_base_urls() -> Vec<String> {
    let mut urls = Vec::new();

    if let Some(value) = env::var("SAICODE_SAI_SEARCH_BASE_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        urls.push(value);
    }

    if env::var("SAICODE_DISABLE_LOCAL_SAI_SEARCH").ok().as_deref() != Some("1")
        && !urls
            .iter()
            .any(|value| value == DEFAULT_LOCAL_SAI_SEARCH_BASE_URL)
    {
        urls.push(DEFAULT_LOCAL_SAI_SEARCH_BASE_URL.to_string());
    }

    urls
}

fn get_sai_search_ssh_target() -> Option<String> {
    if env::var("SAICODE_DISABLE_SAI_SEARCH_SSH").ok().as_deref() == Some("1") {
        return None;
    }
    env::var("SAICODE_SAI_SEARCH_SSH_TARGET")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| Some(DEFAULT_SAI_SEARCH_SSH_TARGET.to_string()))
}

fn is_local_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

fn join_base_url_with_path(base: &str, path: &str) -> String {
    let trimmed_base = base.trim_end_matches('/');
    let trimmed_path = path.trim_start_matches('/');
    format!("{trimmed_base}/{trimmed_path}")
}

fn build_native_webfetch_secondary_prompt(content: &str, prompt: &str) -> String {
    let truncated = if content.chars().count() > DEFAULT_WEB_FETCH_MARKDOWN_LENGTH {
        format!(
            "{}\n\n[Content truncated due to length...]",
            truncate_chars(content, DEFAULT_WEB_FETCH_MARKDOWN_LENGTH)
        )
    } else {
        content.to_string()
    };

    format!(
        "Web page content:\n---\n{}\n---\n\n{}\n\nProvide a concise response based only on the content above. In your response:\n- Enforce a strict 125-character maximum for quotes from any source document. Open Source Software is ok as long as we respect the license.\n- Use quotation marks for exact language from articles; any language outside of the quotation should never be word-for-word the same.\n- You are not a lawyer and never comment on the legality of your own prompts and responses.\n- Never produce or reproduce exact song lyrics.",
        truncated,
        prompt
    )
}

fn run_simple_json_post(
    url: &str,
    payload: &Value,
    user_agent: &str,
    timeout_ms: u64,
) -> Result<Value, String> {
    let mut command = curl_command();
    command
        .arg("-sS")
        .arg("-X")
        .arg("POST")
        .arg(url)
        .arg("--max-time")
        .arg(format!("{:.3}", timeout_ms as f64 / 1000.0))
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-H")
        .arg(format!("User-Agent: {user_agent}"))
        .arg("--data-binary")
        .arg(payload.to_string())
        .arg("--write-out")
        .arg("\n__SAICODE_STATUS__:%{http_code}");

    let output = command
        .output()
        .map_err(|error| format!("Failed to execute curl: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "curl POST failed".to_string()
        } else {
            format!("curl POST failed: {stderr}")
        });
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("curl POST returned invalid UTF-8: {error}"))?;
    let (body, status_code) = split_status_marker(&stdout)?;
    if !(200..300).contains(&status_code) {
        return Err(body);
    }

    serde_json::from_str(&body).map_err(|error| format!("Failed to parse JSON response: {error}"))
}

fn fetch_url_with_curl(
    url: &str,
    accept: &str,
    user_agent: &str,
    timeout_ms: u64,
    follow_redirects: Option<usize>,
) -> Result<HttpFetchResponse, String> {
    let header_path = unique_temp_path("headers", "txt");
    let body_path = unique_temp_path("body", "bin");

    let mut command = curl_command();
    command
        .arg("-sS")
        .arg("-D")
        .arg(&header_path)
        .arg("-o")
        .arg(&body_path)
        .arg("--max-time")
        .arg(format!("{:.3}", timeout_ms as f64 / 1000.0))
        .arg("--max-filesize")
        .arg(MAX_HTTP_CONTENT_LENGTH.to_string())
        .arg("-H")
        .arg(format!("Accept: {accept}"))
        .arg("-H")
        .arg(format!("User-Agent: {user_agent}"))
        .arg(url)
        .arg("--write-out")
        .arg("\n__SAICODE_STATUS__:%{http_code}\n__SAICODE_CONTENT_TYPE__:%{content_type}");

    if let Some(max_redirs) = follow_redirects {
        command
            .arg("-L")
            .arg("--max-redirs")
            .arg(max_redirs.to_string());
    }

    let output = command
        .output()
        .map_err(|error| format!("Failed to execute curl: {error}"))?;
    let headers_text = fs::read_to_string(&header_path).unwrap_or_default();
    let body = fs::read(&body_path).unwrap_or_default();
    let _ = fs::remove_file(&header_path);
    let _ = fs::remove_file(&body_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "curl request failed".to_string()
        } else {
            format!("curl request failed: {stderr}")
        });
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("curl metadata returned invalid UTF-8: {error}"))?;
    let status_code = extract_marker_value(&stdout, "__SAICODE_STATUS__")
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| "Failed to parse curl status code".to_string())?;
    let marker_content_type =
        extract_marker_value(&stdout, "__SAICODE_CONTENT_TYPE__").unwrap_or_default();

    let (_status_text, headers) = parse_last_header_block(&headers_text);
    let content_type = if marker_content_type.is_empty() {
        get_header_value(&headers, "content-type")
            .unwrap_or("")
            .to_string()
    } else {
        marker_content_type
    };

    Ok(HttpFetchResponse {
        status_code,
        content_type,
        headers,
        body,
    })
}

fn curl_command() -> Command {
    if let Some(explicit) = env::var("SAICODE_CURL_BIN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Command::new(explicit);
    }
    if Path::new("/usr/bin/curl").is_file() {
        return Command::new("/usr/bin/curl");
    }
    Command::new("curl")
}

fn unique_temp_path(label: &str, suffix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    env::temp_dir().join(format!(
        "saicode-native-web-{}-{}-{}.{}",
        label,
        std::process::id(),
        unique,
        suffix
    ))
}

fn split_status_marker(stdout: &str) -> Result<(String, u16), String> {
    let marker = "\n__SAICODE_STATUS__:";
    let split_index = stdout
        .rfind(marker)
        .ok_or_else(|| "Failed to parse curl status marker".to_string())?;
    let body = stdout[..split_index].to_string();
    let status_code = stdout[split_index + marker.len()..]
        .trim()
        .parse::<u16>()
        .map_err(|error| format!("Invalid curl status code: {error}"))?;
    Ok((body, status_code))
}

fn extract_marker_value(stdout: &str, marker_name: &str) -> Option<String> {
    let marker = format!("\n{marker_name}:");
    let start = stdout.rfind(&marker)?;
    let value = stdout[start + marker.len()..]
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    Some(value)
}

fn parse_last_header_block(headers_text: &str) -> (String, Vec<HeaderLine>) {
    let normalized = headers_text.replace("\r\n", "\n");
    let mut last_status = String::new();
    let mut last_headers = Vec::new();

    for block in normalized.split("\n\n") {
        let mut lines = block.lines().filter(|line| !line.trim().is_empty());
        let Some(status_line) = lines.next() else {
            continue;
        };
        if !status_line.starts_with("HTTP/") {
            continue;
        }
        let (_, status_text) = parse_status_line(status_line);
        last_status = status_text;
        last_headers = lines
            .filter_map(|line| {
                line.split_once(':').map(|(name, value)| HeaderLine {
                    name: name.trim().to_string(),
                    value: value.trim().to_string(),
                })
            })
            .collect();
    }

    (last_status, last_headers)
}

fn parse_status_line(line: &str) -> (u16, String) {
    let mut parts = line.split_whitespace();
    let _http = parts.next();
    let status_code = parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or_default();
    let status_text = parts.collect::<Vec<_>>().join(" ");
    (status_code, status_text)
}

fn get_header_value<'a>(headers: &'a [HeaderLine], key: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(key))
        .map(|header| header.value.as_str())
}

fn is_redirect_status(status_code: u16) -> bool {
    matches!(status_code, 301 | 302 | 307 | 308)
}

fn is_textual_content_type(content_type: &str) -> bool {
    let normalized = content_type.to_ascii_lowercase();
    normalized.is_empty()
        || normalized.contains("text/")
        || normalized.contains("application/json")
        || normalized.contains("application/xml")
        || normalized.contains("application/xhtml")
        || normalized.contains("application/javascript")
        || normalized.contains("application/x-javascript")
}

fn resolve_redirect_url(base_url: &str, location: &str) -> Option<String> {
    let location = location.trim();
    if location.starts_with("https://") || location.starts_with("http://") {
        return Some(location.to_string());
    }

    let base = parse_http_url(base_url)?;
    if location.starts_with("//") {
        return Some(format!("{}:{}", base.scheme, location));
    }
    if location.starts_with('/') {
        return Some(format!("{}{}", base.origin(), location));
    }
    if location.starts_with('?') {
        return Some(format!(
            "{}{}{}",
            base.origin(),
            base.path_without_query(),
            location
        ));
    }

    Some(format!(
        "{}{}",
        base.origin(),
        normalize_relative_path(&base.directory_path(), location)
    ))
}

fn normalize_relative_path(base_dir: &str, relative: &str) -> String {
    let (relative_path, suffix) = split_path_suffix(relative);
    let mut segments: Vec<&str> = base_dir
        .trim_start_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    for segment in relative_path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            _ => segments.push(segment),
        }
    }

    let normalized_path = if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    };
    format!("{normalized_path}{suffix}")
}

fn split_path_suffix(input: &str) -> (&str, &str) {
    if let Some(index) = input.find(|ch| ch == '?' || ch == '#') {
        (&input[..index], &input[index..])
    } else {
        (input, "")
    }
}

fn is_permitted_redirect(original_url: &str, redirect_url: &str) -> bool {
    let Some(original) = parse_http_url(original_url) else {
        return false;
    };
    let Some(redirect) = parse_http_url(redirect_url) else {
        return false;
    };

    original.scheme == redirect.scheme
        && original.port == redirect.port
        && strip_www(&original.host) == strip_www(&redirect.host)
}

fn strip_www(host: &str) -> &str {
    host.strip_prefix("www.").unwrap_or(host)
}

fn extract_sai_search_hits(response: &Value) -> Result<Vec<SearchHit>, String> {
    let Some(results) = response.get("results").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };

    Ok(results
        .iter()
        .filter_map(|item| {
            let title = item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            let url = item.get("url").and_then(Value::as_str).unwrap_or("").trim();
            if title.is_empty() || url.is_empty() {
                return None;
            }
            Some(SearchHit {
                title: title.to_string(),
                url: url.to_string(),
            })
        })
        .collect())
}

fn normalize_hits(
    query: &str,
    hits: Vec<SearchHit>,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Vec<SearchHit> {
    let mut normalized = dedupe_hits(hits);
    if !allowed_domains.is_empty() {
        normalized.retain(|hit| host_matches_list(&hit.url, allowed_domains));
    }
    if !blocked_domains.is_empty() {
        normalized.retain(|hit| !host_matches_list(&hit.url, blocked_domains));
    }
    if let Some(domain) = normalize_query_domain(query) {
        if !normalized.iter().any(|hit| {
            parse_http_url(&hit.url)
                .map(|parsed| parsed.host == domain)
                .unwrap_or(false)
        }) {
            normalized.insert(
                0,
                SearchHit {
                    title: domain.clone(),
                    url: format!("https://{domain}"),
                },
            );
        }
        normalized.sort_by_key(|hit| rank_search_hit_for_domain(&domain, hit));
    }
    normalized.truncate(DEFAULT_MAX_SEARCH_RESULTS);
    normalized
}

fn normalize_query_domain(query: &str) -> Option<String> {
    let trimmed = query.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() || trimmed.contains(char::is_whitespace) {
        return None;
    }
    let candidate = trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(trimmed)
        .trim()
        .trim_start_matches('.')
        .trim_end_matches('/')
        .to_ascii_lowercase();
    (candidate.contains('.') && candidate.chars().any(|ch| ch.is_ascii_alphabetic()))
        .then_some(candidate)
}

fn rank_search_hit_for_domain(domain: &str, hit: &SearchHit) -> u8 {
    let url_host = parse_http_url(&hit.url).map(|parsed| parsed.host);
    if url_host.as_deref() == Some(domain) {
        return 0;
    }
    let lowered_title = hit.title.to_ascii_lowercase();
    let lowered_url = hit.url.to_ascii_lowercase();
    if lowered_title.contains(domain) || lowered_url.contains(domain) {
        return 1;
    }
    2
}

fn dedupe_hits(hits: Vec<SearchHit>) -> Vec<SearchHit> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for hit in hits {
        let key = hit.url.trim().to_ascii_lowercase();
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        deduped.push(hit);
    }
    deduped
}

fn host_matches_list(url: &str, domains: &[String]) -> bool {
    let Some(parsed) = parse_http_url(url) else {
        return false;
    };
    domains.iter().any(|domain| {
        let normalized = normalize_domain_filter(domain);
        parsed.host == normalized || parsed.host.ends_with(&format!(".{normalized}"))
    })
}

fn normalize_domain_filter(domain: &str) -> String {
    let trimmed = domain.trim();
    if let Some(parsed) = parse_http_url(trimmed) {
        return parsed.host;
    }

    trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(trimmed)
        .trim()
        .trim_start_matches('.')
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn fetch_top_page_excerpts(query: &str, hits: &[SearchHit]) -> Vec<FetchedPage> {
    let top_k = get_search_fetch_top_k();
    if top_k == 0 {
        return Vec::new();
    }

    hits.iter()
        .take(top_k)
        .filter_map(|hit| fetch_page_excerpt(query, hit))
        .collect()
}

fn get_search_fetch_top_k() -> usize {
    env::var("SAICODE_WEB_SEARCH_FETCH_TOP_K")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map(|value| value.min(DEFAULT_MAX_FETCHED_PAGES))
        .unwrap_or(DEFAULT_MAX_FETCHED_PAGES)
}

fn fetch_page_excerpt(query: &str, hit: &SearchHit) -> Option<FetchedPage> {
    let response = fetch_url_with_curl(
        &hit.url,
        "text/html, text/plain, text/markdown;q=0.9, */*;q=0.8",
        SEARCH_USER_AGENT,
        DEFAULT_PAGE_FETCH_TIMEOUT_MS,
        Some(5),
    )
    .ok()?;
    if !is_textual_content_type(&response.content_type) || response.body.contains(&0) {
        return None;
    }

    let raw = String::from_utf8_lossy(&response.body).to_string();
    let text = if response
        .content_type
        .to_ascii_lowercase()
        .contains("text/html")
    {
        html_to_text(&extract_primary_html_region(&raw))
    } else {
        collapse_whitespace(&raw)
    };
    let excerpt = build_excerpt_from_text(query, &text);
    if excerpt.is_empty() {
        return None;
    }
    Some(FetchedPage {
        title: hit.title.clone(),
        url: hit.url.clone(),
        excerpt,
    })
}

fn build_search_summary(query: &str, hits: &[SearchHit]) -> String {
    if hits.is_empty() {
        return format!("No web search results matched the query \"{query}\".");
    }

    let rendered_hits = hits
        .iter()
        .map(|hit| format!("- [{}]({})", hit.title, hit.url))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Search results for \"{query}\". Include a Sources section in the final answer.\n{rendered_hits}"
    )
}

fn build_fetched_pages_summary(query: &str, fetched_pages: &[FetchedPage]) -> Option<String> {
    if fetched_pages.is_empty() {
        return None;
    }

    let rendered = fetched_pages
        .iter()
        .enumerate()
        .map(|(index, page)| {
            format!(
                "{}. [{}]({})\n   {}",
                index + 1,
                page.title,
                page.url,
                page.excerpt
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!(
        "Top page excerpts automatically fetched for \"{query}\":\n{rendered}"
    ))
}

fn build_excerpt_from_text(query: &str, text: &str) -> String {
    let normalized = truncate_chars(&collapse_whitespace(text), DEFAULT_PAGE_TEXT_LENGTH);
    if normalized.is_empty() {
        return String::new();
    }

    let query_lower = query.to_ascii_lowercase();
    let terms = extract_query_terms(query);
    let segments: Vec<String> = normalized
        .split(|ch: char| matches!(ch, '.' | '!' | '?' | '\n' | '。' | '！' | '？'))
        .map(str::trim)
        .filter(|segment| segment.chars().count() >= 40)
        .map(ToOwned::to_owned)
        .collect();

    let mut best_segment = String::new();
    let mut best_score = i32::MIN;
    for segment in segments {
        let score = score_excerpt_segment(&segment, &query_lower, &terms);
        if score > best_score {
            best_score = score;
            best_segment = segment;
        }
    }

    let source = if best_segment.is_empty() {
        normalized
    } else {
        best_segment
    };
    let truncated = truncate_chars(&source, DEFAULT_EXCERPT_LENGTH);
    if source.chars().count() > DEFAULT_EXCERPT_LENGTH {
        format!("{}...", truncated.trim())
    } else {
        truncated.trim().to_string()
    }
}

fn extract_query_terms(query: &str) -> Vec<String> {
    let stop_words = [
        "the", "and", "for", "with", "from", "that", "this", "what", "when", "where", "which",
        "into", "about", "latest", "official",
    ];

    let mut seen = BTreeSet::new();
    query
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|term| term.len() >= 3)
        .filter(|term| !stop_words.iter().any(|stop| stop == term))
        .filter(|term| seen.insert((*term).to_string()))
        .map(ToOwned::to_owned)
        .collect()
}

fn score_excerpt_segment(segment: &str, query_lower: &str, terms: &[String]) -> i32 {
    let normalized = segment.to_ascii_lowercase();
    let mut score = 0;
    if !query_lower.is_empty() && normalized.contains(query_lower) {
        score += 8;
    }
    for term in terms {
        if normalized.contains(term) {
            score += 2;
        }
    }
    score
}

fn extract_search_hits(html: &str) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    let mut remaining = html;

    while let Some(anchor_start) = remaining.find("result__a") {
        let after_class = &remaining[anchor_start..];
        let Some(href_index) = after_class.find("href=") else {
            remaining = &after_class[1..];
            continue;
        };
        let href_slice = &after_class[href_index + 5..];
        let Some((href, rest)) = extract_quoted_value(href_slice) else {
            remaining = &after_class[1..];
            continue;
        };
        let Some(close_tag_index) = rest.find('>') else {
            remaining = &after_class[1..];
            continue;
        };
        let after_tag = &rest[close_tag_index + 1..];
        let Some(end_anchor_index) = after_tag.find("</a>") else {
            remaining = after_tag;
            continue;
        };
        if let Some(decoded_url) = decode_duckduckgo_redirect(&href) {
            hits.push(SearchHit {
                title: html_to_text(&after_tag[..end_anchor_index]),
                url: decoded_url,
            });
        }
        remaining = &after_tag[end_anchor_index + 4..];
    }

    hits
}

fn extract_search_hits_from_generic_links(html: &str) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    let mut remaining = html;

    while let Some(anchor_start) = remaining.find("<a") {
        let after_anchor = &remaining[anchor_start..];
        let Some(href_index) = after_anchor.find("href=") else {
            remaining = &after_anchor[2..];
            continue;
        };
        let href_slice = &after_anchor[href_index + 5..];
        let Some((href, rest)) = extract_quoted_value(href_slice) else {
            remaining = &after_anchor[2..];
            continue;
        };
        let Some(close_tag_index) = rest.find('>') else {
            remaining = &after_anchor[2..];
            continue;
        };
        let after_tag = &rest[close_tag_index + 1..];
        let Some(end_anchor_index) = after_tag.find("</a>") else {
            remaining = &after_anchor[2..];
            continue;
        };
        let title = html_to_text(&after_tag[..end_anchor_index]);
        if let Some(decoded_url) = decode_duckduckgo_redirect(&href) {
            if should_keep_generic_hit(&decoded_url, &title) {
                hits.push(SearchHit {
                    title,
                    url: decoded_url,
                });
            }
        }
        remaining = &after_tag[end_anchor_index + 4..];
    }

    hits
}

fn extract_quoted_value(input: &str) -> Option<(String, &str)> {
    let quote = input.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &input[1..];
    let end = rest.find(quote)?;
    Some((rest[..end].to_string(), &rest[end + 1..]))
}

fn decode_duckduckgo_redirect(url: &str) -> Option<String> {
    let normalized_url = if url.starts_with("//") {
        format!("https:{url}")
    } else if url.starts_with('/') {
        format!("https://duckduckgo.com{url}")
    } else {
        url.to_string()
    };

    let parsed = parse_http_url(&normalized_url)?;
    if parsed.host.ends_with("duckduckgo.com")
        && matches!(parsed.path_without_query(), "/l" | "/l/")
    {
        let redirected = extract_query_param(&parsed.path_and_query, "uddg")?;
        return Some(decode_html_entities(&percent_decode(&redirected)));
    }

    Some(decode_html_entities(&normalized_url))
}

fn extract_query_param(path_and_query: &str, key: &str) -> Option<String> {
    let query = path_and_query.split_once('?')?.1;
    for part in query.split('&') {
        let (name, value) = part.split_once('=').unwrap_or((part, ""));
        if name == key {
            return Some(value.to_string());
        }
    }
    None
}

fn should_keep_generic_hit(url: &str, title: &str) -> bool {
    if title.trim().is_empty() {
        return false;
    }
    let Some(parsed) = parse_http_url(url) else {
        return false;
    };
    !matches!(
        parsed.host.as_str(),
        "duckduckgo.com" | "html.duckduckgo.com" | "duckduckgo.onion"
    )
}

fn html_to_text(html: &str) -> String {
    let without_script = strip_tag_block_case_insensitive(html, "script");
    let without_style = strip_tag_block_case_insensitive(&without_script, "style");
    let mut output = String::new();
    let mut inside_tag = false;

    for ch in without_style.chars() {
        match ch {
            '<' => {
                inside_tag = true;
                output.push(' ');
            }
            '>' => {
                inside_tag = false;
                output.push(' ');
            }
            _ if !inside_tag => output.push(ch),
            _ => {}
        }
    }

    collapse_whitespace(&decode_html_entities(&output))
}

fn extract_primary_html_region(html: &str) -> String {
    extract_tag_inner_case_insensitive(html, "main")
        .or_else(|| extract_tag_inner_case_insensitive(html, "article"))
        .or_else(|| extract_tag_inner_case_insensitive(html, "body"))
        .unwrap_or_else(|| html.to_string())
}

fn extract_tag_inner_case_insensitive(html: &str, tag: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = lower.find(&open)?;
    let after_open = html[start..].find('>')? + start + 1;
    let end = lower[after_open..].find(&close)? + after_open;
    Some(html[after_open..end].to_string())
}

fn strip_tag_block_case_insensitive(html: &str, tag: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut cursor = 0usize;
    let mut output = String::new();

    while let Some(rel_start) = lower[cursor..].find(&open) {
        let start = cursor + rel_start;
        output.push_str(&html[cursor..start]);
        if let Some(rel_end) = lower[start..].find(&close) {
            let end = start + rel_end + close.len();
            cursor = end;
        } else {
            cursor = html.len();
            break;
        }
        output.push(' ');
    }

    output.push_str(&html[cursor..]);
    output
}

fn collapse_whitespace(text: &str) -> String {
    let mut output = String::new();
    let mut previous_was_whitespace = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !previous_was_whitespace {
                output.push(' ');
                previous_was_whitespace = true;
            }
        } else {
            output.push(ch);
            previous_was_whitespace = false;
        }
    }
    output.trim().to_string()
}

fn decode_html_entities(text: &str) -> String {
    let mut output = String::new();
    let bytes = text.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'&' {
            if let Some(rel_end) = text[index..].find(';') {
                let end = index + rel_end;
                let entity = &text[index + 1..end];
                if let Some(decoded) = decode_html_entity(entity) {
                    output.push(decoded);
                    index = end + 1;
                    continue;
                }
            }
        }
        output.push(bytes[index] as char);
        index += 1;
    }

    output
}

fn decode_html_entity(entity: &str) -> Option<char> {
    match entity.to_ascii_lowercase().as_str() {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        "nbsp" => Some(' '),
        value if value.starts_with("#x") => u32::from_str_radix(&value[2..], 16)
            .ok()
            .and_then(char::from_u32),
        value if value.starts_with('#') => value[1..].parse::<u32>().ok().and_then(char::from_u32),
        _ => None,
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hi = (bytes[index + 1] as char).to_digit(16);
                let lo = (bytes[index + 2] as char).to_digit(16);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    output.push(((hi << 4) + lo) as u8);
                    index += 3;
                    continue;
                }
                output.push(bytes[index]);
                index += 1;
            }
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            other => {
                output.push(other);
                index += 1;
            }
        }
    }

    String::from_utf8_lossy(&output).to_string()
}

fn url_encode_component(input: &str) -> String {
    let mut output = String::new();
    for byte in input.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~') {
            output.push(*byte as char);
        } else if *byte == b' ' {
            output.push('+');
        } else {
            output.push('%');
            output.push_str(&format!("{:02X}", byte));
        }
    }
    output
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        build_search_url, decode_duckduckgo_redirect, determine_bash_policy, execute_bash_tool,
        execute_edit_tool, execute_grep_tool, execute_read_tool, execute_write_tool,
        extract_search_hits, format_web_search_tool_output, get_sai_search_base_urls,
        is_permitted_redirect, normalize_relative_path, normalize_tool_restriction_values,
        parse_http_url, parse_local_tools_args, resolve_native_local_tools_model,
        resolve_path_within_cwd, resolve_redirect_url, split_shell_segments,
        validate_readonly_bash_command, FetchedPage, NativeBashPolicy, NativeToolSession,
        NativeWebSearchOutput, SearchHit, ToolExecution,
    };
    use serde_json::json;
    use std::env;
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use std::thread;
    use std::time::Duration;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let path = env::temp_dir().join(format!("saicode-native-local-tools-{label}-{unique}"));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn session(root: &std::path::Path) -> NativeToolSession {
        NativeToolSession::new(root.to_path_buf(), NativeBashPolicy::ReadOnly)
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock")
    }

    #[test]
    fn parses_supported_native_local_tool_flags() {
        let parsed = parse_local_tools_args(&args(&[
            "-p",
            "--allowedTools",
            "Read,Grep,Bash",
            "--model",
            "cpa/gpt-5.4",
            "--max-turns",
            "5",
            "hello",
        ]))
        .expect("parse should succeed");

        assert!(parsed.print);
        assert_eq!(parsed.tools, vec!["Read", "Grep", "Bash"]);
        assert_eq!(parsed.max_turns, 5);
        assert_eq!(parsed.prompt, "hello");
    }

    #[test]
    fn parses_native_local_tools_with_double_dash_prompt_separator() {
        let parsed = parse_local_tools_args(&args(&[
            "-p",
            "--allowedTools",
            "Read",
            "--",
            "hello",
            "world",
        ]))
        .expect("parse should succeed");

        assert_eq!(parsed.tools, vec!["Read"]);
        assert_eq!(parsed.prompt, "hello world");
    }

    #[test]
    fn parses_native_bash_bypass_flags() {
        let parsed = parse_local_tools_args(&args(&[
            "-p",
            "--tools",
            "Bash",
            "--permission-mode",
            "bypassPermissions",
            "--dangerously-skip-permissions",
            "hello",
        ]))
        .expect("parse should succeed");

        assert_eq!(parsed.permission_mode.as_deref(), Some("bypassPermissions"));
        assert!(parsed.dangerously_skip_permissions);
        assert_eq!(
            determine_bash_policy(&parsed),
            NativeBashPolicy::Unrestricted
        );
    }

    #[test]
    fn native_local_tools_default_model_prefers_small_fast_path() {
        let _guard = env_lock();
        env::remove_var("SAICODE_NATIVE_LOCAL_TOOLS_MODEL");
        env::remove_var("SAICODE_SMALL_FAST_MODEL");
        env::remove_var("SAICODE_DEFAULT_HAIKU_MODEL");

        let resolved = resolve_native_local_tools_model(None);
        assert_eq!(resolved.alias, "cpa/gpt-5.4-mini");
        assert_eq!(resolved.model, "gpt-5.4-mini");

        env::set_var("SAICODE_NATIVE_LOCAL_TOOLS_MODEL", "cpa/qwen3-coder-plus");
        let resolved = resolve_native_local_tools_model(None);
        assert_eq!(resolved.alias, "cpa/qwen3-coder-plus");

        let resolved = resolve_native_local_tools_model(Some("cpa/gpt-5.4"));
        assert_eq!(resolved.alias, "cpa/gpt-5.4");

        env::remove_var("SAICODE_NATIVE_LOCAL_TOOLS_MODEL");
    }

    #[test]
    fn rejects_bash_matcher_suffix_rules() {
        let error = parse_local_tools_args(&args(&[
            "-p",
            "--allowedTools",
            "Bash(git status:*)",
            "hello",
        ]))
        .expect_err("Bash matcher suffix should be rejected");

        assert!(error.contains("does not support Bash(...)"));
    }

    #[test]
    fn normalizes_tool_values() {
        let values =
            normalize_tool_restriction_values(&args(&["Read,Grep", "Glob(extra),WebSearch"]));
        assert_eq!(values, vec!["Read", "Grep", "Glob", "WebSearch"]);
    }

    #[test]
    fn read_tool_formats_numbered_output() {
        let root = temp_dir("read");
        let file = root.join("sample.txt");
        fs::write(&file, "alpha\nbeta\ngamma\n").expect("write sample");
        let mut session = session(&root);

        let output = execute_read_tool(
            &json!({ "file_path": file.to_string_lossy(), "offset": 2, "limit": 2 }),
            &mut session,
        );

        match output {
            ToolExecution::Output(text) => {
                assert!(text.contains("sample.txt"));
                assert!(text.contains("2\tbeta"));
                assert!(text.contains("3\tgamma"));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("read tool unexpectedly requested fallback: {reason}");
            }
        }
    }

    #[test]
    fn grep_defaults_to_content_mode_for_single_file_paths() {
        let root = temp_dir("grep-file-default");
        let file = root.join("sample.txt");
        fs::write(&file, "alpha\nmembers = [\nomega\n").expect("write sample");

        let output = execute_grep_tool(
            &json!({
                "pattern": "^members\\s*=",
                "path": "sample.txt"
            }),
            &root,
        );

        match output {
            ToolExecution::Output(text) => {
                assert!(text.contains("members = ["));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("grep unexpectedly requested fallback: {reason}");
            }
        }
    }

    #[test]
    fn grep_accepts_text_output_mode_alias_and_ignores_file_type_hint() {
        let root = temp_dir("grep-text-alias");
        let file = root.join("sample.txt");
        fs::write(&file, "alpha\nmembers = [\nomega\n").expect("write sample");

        let output = execute_grep_tool(
            &json!({
                "pattern": "members\\s*=",
                "path": "sample.txt",
                "output_mode": "text",
                "type": "f"
            }),
            &root,
        );

        match output {
            ToolExecution::Output(text) => {
                assert!(text.contains("members = ["));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("grep unexpectedly requested fallback: {reason}");
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn grep_accepts_relative_file_paths_under_symlinked_cwd() {
        let root = temp_dir("grep-symlink-target");
        let alias_parent = temp_dir("grep-symlink-parent");
        let alias = alias_parent.join("workspace-link");
        std::os::unix::fs::symlink(&root, &alias).expect("create cwd symlink");
        let file = root.join("sample.txt");
        fs::write(&file, "alpha\nmembers = [\nomega\n").expect("write sample");

        let output = execute_grep_tool(
            &json!({
                "pattern": "^members\\s*=",
                "path": "sample.txt"
            }),
            &alias,
        );

        match output {
            ToolExecution::Output(text) => {
                assert!(text.contains("members = ["));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("grep unexpectedly requested fallback: {reason}");
            }
        }
    }

    #[test]
    fn edit_tool_bootstraps_snapshot_for_existing_file() {
        let root = temp_dir("edit-without-read");
        let file = root.join("sample.txt");
        fs::write(&file, "alpha\n").expect("write sample");
        let mut session = session(&root);

        let output = execute_edit_tool(
            &json!({
                "file_path": file.to_string_lossy(),
                "old_string": "alpha",
                "new_string": "beta"
            }),
            &mut session,
        );

        match output {
            ToolExecution::Output(text) => {
                assert!(text.contains("updated successfully"));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("edit unexpectedly requested fallback: {reason}");
            }
        }

        let contents = fs::read_to_string(&file).expect("read edited file");
        assert_eq!(contents, "beta\n");
    }

    #[test]
    fn write_tool_creates_new_file() {
        let root = temp_dir("write-create");
        let file = root.join("created.txt");
        let mut session = session(&root);

        let output = execute_write_tool(
            &json!({ "file_path": "created.txt", "content": "hello\nworld\n" }),
            &mut session,
        );

        match output {
            ToolExecution::Output(text) => {
                assert!(text.contains("File created successfully"));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("write tool unexpectedly requested fallback: {reason}");
            }
        }

        assert_eq!(
            fs::read_to_string(&file).expect("read created file"),
            "hello\nworld\n"
        );
    }

    #[test]
    fn write_tool_requires_full_read_for_existing_file() {
        let root = temp_dir("write-update");
        let file = root.join("sample.txt");
        fs::write(&file, "alpha\nbeta\n").expect("write sample");
        let mut session = session(&root);

        let without_read = execute_write_tool(
            &json!({ "file_path": "sample.txt", "content": "gamma\n" }),
            &mut session,
        );
        match without_read {
            ToolExecution::Output(text) => {
                assert!(text.contains("has not been read yet"));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("write tool unexpectedly requested fallback: {reason}");
            }
        }

        let _ = execute_read_tool(
            &json!({ "file_path": "sample.txt", "offset": 1, "limit": 1 }),
            &mut session,
        );
        let partial = execute_write_tool(
            &json!({ "file_path": "sample.txt", "content": "gamma\n" }),
            &mut session,
        );
        match partial {
            ToolExecution::Output(text) => {
                assert!(text.contains("partially read"));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("write tool unexpectedly requested fallback: {reason}");
            }
        }

        let _ = execute_read_tool(&json!({ "file_path": "sample.txt" }), &mut session);
        let success = execute_write_tool(
            &json!({ "file_path": "sample.txt", "content": "gamma\n" }),
            &mut session,
        );
        match success {
            ToolExecution::Output(text) => {
                assert!(text.contains("updated successfully"));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("write tool unexpectedly requested fallback: {reason}");
            }
        }

        assert_eq!(
            fs::read_to_string(&file).expect("read updated file"),
            "gamma\n"
        );
    }

    #[test]
    fn edit_tool_replaces_unique_match_and_preserves_quote_style() {
        let root = temp_dir("edit-unique");
        let file = root.join("sample.txt");
        fs::write(&file, "say “hello” now\n").expect("write sample");
        let mut session = session(&root);

        let _ = execute_read_tool(&json!({ "file_path": "sample.txt" }), &mut session);
        let output = execute_edit_tool(
            &json!({
                "file_path": "sample.txt",
                "old_string": "\"hello\"",
                "new_string": "\"goodbye\""
            }),
            &mut session,
        );

        match output {
            ToolExecution::Output(text) => {
                assert!(text.contains("updated successfully"));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("edit tool unexpectedly requested fallback: {reason}");
            }
        }

        assert_eq!(
            fs::read_to_string(&file).expect("read edited file"),
            "say “goodbye” now\n"
        );
    }

    #[test]
    fn edit_tool_rejects_multiple_matches_without_replace_all() {
        let root = temp_dir("edit-multi");
        let file = root.join("sample.txt");
        fs::write(&file, "alpha\nalpha\n").expect("write sample");
        let mut session = session(&root);

        let _ = execute_read_tool(&json!({ "file_path": "sample.txt" }), &mut session);
        let output = execute_edit_tool(
            &json!({
                "file_path": "sample.txt",
                "old_string": "alpha",
                "new_string": "beta"
            }),
            &mut session,
        );

        match output {
            ToolExecution::Output(text) => {
                assert!(text.contains("Found 2 matches"));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("edit tool unexpectedly requested fallback: {reason}");
            }
        }
    }

    #[test]
    fn edit_tool_detects_stale_file_after_read() {
        let root = temp_dir("edit-stale");
        let file = root.join("sample.txt");
        fs::write(&file, "alpha\n").expect("write sample");
        let mut session = session(&root);

        let _ = execute_read_tool(&json!({ "file_path": "sample.txt" }), &mut session);
        thread::sleep(Duration::from_millis(5));
        fs::write(&file, "beta\n").expect("mutate file after read");

        let output = execute_edit_tool(
            &json!({
                "file_path": "sample.txt",
                "old_string": "alpha",
                "new_string": "gamma"
            }),
            &mut session,
        );

        match output {
            ToolExecution::Output(text) => {
                assert!(text.contains("modified since read"));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("edit tool unexpectedly requested fallback: {reason}");
            }
        }
    }

    #[test]
    fn rejects_paths_outside_cwd() {
        let root = temp_dir("cwd");
        let outside_parent = temp_dir("outside-parent");
        let outside = outside_parent.join("outside.txt");
        fs::write(&outside, "nope").expect("write outside file");

        let error = resolve_path_within_cwd(&root, &outside.to_string_lossy(), false)
            .expect_err("path should be rejected");
        assert!(error.contains("outside the current working directory"));
    }

    #[test]
    fn validates_readonly_bash_subset() {
        validate_readonly_bash_command("pwd && command -v rg")
            .expect("readonly bash command should be accepted");
        let error = validate_readonly_bash_command("echo hi > /tmp/test.txt")
            .expect_err("write command should be rejected");
        assert!(error.contains("shell syntax"));
    }

    #[test]
    fn splits_shell_segments_with_quotes() {
        let segments = split_shell_segments(r#"printf "a|b" | head -n 1 && pwd"#)
            .expect("segments should parse");
        assert_eq!(segments, vec![r#"printf "a|b""#, "head -n 1", "pwd"]);
    }

    #[test]
    fn runs_native_bash_in_temp_cwd() {
        let root = temp_dir("bash");
        let mut session = session(&root);
        let output = execute_bash_tool(&json!({ "command": "pwd" }), &mut session);
        match output {
            ToolExecution::Output(text) => {
                assert!(text.contains("exit_code: 0"));
                assert!(text.contains(root.to_string_lossy().as_ref()));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("bash tool unexpectedly requested fallback: {reason}");
            }
        }
    }

    #[test]
    fn runs_native_bash_write_in_bypass_mode() {
        let root = temp_dir("bash-bypass");
        let file = root.join("from-bash.txt");
        let mut session = NativeToolSession::new(root.clone(), NativeBashPolicy::Unrestricted);

        let output = execute_bash_tool(
            &json!({ "command": "printf 'hello-from-bash\\n' > from-bash.txt" }),
            &mut session,
        );

        match output {
            ToolExecution::Output(text) => {
                assert!(text.contains("exit_code: 0"));
            }
            ToolExecution::FallbackToRustFullCli(reason) => {
                panic!("bypass bash unexpectedly requested fallback: {reason}");
            }
        }

        assert_eq!(
            fs::read_to_string(&file).expect("read bash output file"),
            "hello-from-bash\n"
        );
    }

    #[test]
    fn native_bash_still_falls_back_for_background_mode() {
        let root = temp_dir("bash-background");
        let mut session = NativeToolSession::new(root, NativeBashPolicy::Unrestricted);

        let output = execute_bash_tool(
            &json!({ "command": "sleep 1", "run_in_background": true }),
            &mut session,
        );

        match output {
            ToolExecution::FallbackToRustFullCli(reason) => {
                assert!(reason.contains("run_in_background"));
            }
            ToolExecution::Output(text) => {
                panic!("background bash should have fallen back, got output: {text}");
            }
        }
    }

    #[test]
    fn parses_http_urls_and_relative_redirects() {
        let parsed = parse_http_url("https://example.com/docs/page?q=1").expect("url should parse");
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.path_without_query(), "/docs/page");
        assert_eq!(
            resolve_redirect_url("https://example.com/docs/page", "../next?x=1"),
            Some("https://example.com/next?x=1".to_string())
        );
        assert_eq!(
            normalize_relative_path("/docs/sub/", "../next"),
            "/docs/next"
        );
    }

    #[test]
    fn only_permits_same_host_redirects() {
        assert!(is_permitted_redirect(
            "https://example.com/docs",
            "https://www.example.com/docs/next"
        ));
        assert!(!is_permitted_redirect(
            "https://example.com/docs",
            "https://other.example.net/docs"
        ));
    }

    #[test]
    fn decodes_duckduckgo_redirect_links() {
        let url = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fdocs%3Fa%3D1%26b%3D2";
        assert_eq!(
            decode_duckduckgo_redirect(url),
            Some("https://example.com/docs?a=1&b=2".to_string())
        );
    }

    #[test]
    fn extracts_duckduckgo_search_hits() {
        let html = r#"
        <div class="result">
          <a class="result__a" href="https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fguide">Example Guide</a>
        </div>
        "#;
        let hits = extract_search_hits(html);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Example Guide");
        assert_eq!(hits[0].url, "https://example.com/guide");
    }

    #[test]
    fn formats_native_web_search_output_like_tool_result() {
        let output = NativeWebSearchOutput {
            query: "rust docs".to_string(),
            hits: vec![SearchHit {
                title: "Rust".to_string(),
                url: "https://www.rust-lang.org".to_string(),
            }],
            fetched_pages: vec![FetchedPage {
                title: "Rust".to_string(),
                url: "https://www.rust-lang.org".to_string(),
                excerpt: "Systems programming language.".to_string(),
            }],
        };

        let rendered = format_web_search_tool_output(&output);
        assert!(rendered.contains("Web search results for query: \"rust docs\""));
        assert!(rendered.contains("[Rust](https://www.rust-lang.org)"));
        assert!(rendered.contains("Sources"));
        assert!(rendered.contains("\"title\":\"Rust\""));
    }

    #[test]
    fn builds_search_urls_with_query_parameter() {
        let url = build_search_url("rust async");
        assert!(url.contains("q=rust+async") || url.contains("q=rust%20async"));
    }

    #[test]
    fn sai_search_base_urls_prefer_env_then_local_default() {
        let _guard = env_lock();
        env::remove_var("SAICODE_SAI_SEARCH_BASE_URL");
        env::remove_var("SAICODE_DISABLE_LOCAL_SAI_SEARCH");

        let urls = get_sai_search_base_urls();
        assert_eq!(urls, vec!["http://127.0.0.1:18961".to_string()]);

        env::set_var(
            "SAICODE_SAI_SEARCH_BASE_URL",
            "http://search.internal:18961",
        );
        let urls = get_sai_search_base_urls();
        assert_eq!(
            urls,
            vec![
                "http://search.internal:18961".to_string(),
                "http://127.0.0.1:18961".to_string()
            ]
        );

        env::set_var("SAICODE_DISABLE_LOCAL_SAI_SEARCH", "1");
        let urls = get_sai_search_base_urls();
        assert_eq!(urls, vec!["http://search.internal:18961".to_string()]);

        env::remove_var("SAICODE_SAI_SEARCH_BASE_URL");
        env::remove_var("SAICODE_DISABLE_LOCAL_SAI_SEARCH");
    }
}
