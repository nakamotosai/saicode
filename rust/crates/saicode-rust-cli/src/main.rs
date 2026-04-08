use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use api::{
    ApiError, ContentBlockDelta, InputContentBlock, InputMessage, MessageRequest,
    OpenAiCompatClient, OpenAiCompatConfig, OutputContentBlock, StreamEvent as ApiStreamEvent,
    ToolChoice, ToolDefinition, ToolResultContentBlock,
};
use commands::{
    build_command_registry_snapshot, handle_agents_slash_command, handle_mcp_slash_command,
    handle_plugins_slash_command, handle_skills_slash_command, render_slash_command_help,
    validate_slash_command_input, CommandRegistryContext, PluginsCommandResult, SlashCommand,
};
use plugins::{PluginManager, PluginManagerConfig};
use runtime::{
    compact_session, load_system_prompt, load_system_prompt_bare, load_user_memories,
    resolve_sandbox_status, ApiClient, ApiRequest, AssistantEvent, ContentBlock,
    ConversationMessage, ConversationRuntime, EnforcementResult, ManagedMcpTool, McpServerManager,
    MessageRole, PermissionEnforcer, PermissionMode, PermissionPolicy, PermissionPromptDecision,
    PermissionPrompter, PermissionRequest, ProfileResolver, ProviderLauncher,
    ResolvedPermissionMode, RuntimeConfig, RuntimeError, Session, ToolError, ToolExecutor,
    UsageTracker,
};
use saicode_frontline::recovery::{
    get_provider_config, is_degraded_function_invocation_error_text, resolve_model, WireApi,
    DEFAULT_FAST_FALLBACK_MODEL_ID,
};
use serde_json::{json, Map, Value};
use tools::GlobalToolRegistry;

mod tui;

const VERSION_TEXT: &str = "1.0.0 (saicode)";
const DEFAULT_MODEL: &str = "cpa/qwen/qwen3.5-122b-a10b";
const SESSION_FILE_SUFFIX: &str = ".jsonl";
const NO_CONTENT_MESSAGE: &str = "(no content)";
const HELP_TEXT: &str = r#"Usage: saicode [options] [command] [prompt]

saicode - starts an interactive session by default, use -p/--print for
non-interactive output

Arguments:
  prompt                                            Your prompt

Options:
  --add-dir <directories...>                        Additional directories to allow tool access to
  --agent <agent>                                   Agent for the current session. Overrides the 'agent' setting.
  --agents <json>                                   JSON object defining custom agents
  --allow-dangerously-skip-permissions              Allow Full Access to appear as a selectable option without enabling it by default
  --allowedTools, --allowed-tools <tools...>        Comma or space-separated list of tool names to allow
  --append-system-prompt <prompt>                   Append a system prompt to the default system prompt
  --bare                                            Minimal mode
  -c, --continue                                    Continue the most recent conversation in the current directory
  --dangerously-skip-permissions                    Enable Full Access mode
  --effort <level>                                  Effort level for the current session
  -h, --help                                        Display help for command
  --mcp-config <configs...>                         Load MCP servers from JSON files or strings
  --model <model>                                   Model for the current session
  --no-session-persistence                          Disable session persistence
  --output-format <format>                          Output format for --print (text|json|stream-json)
  --permission-mode <mode>                          Permission mode for the session
  -p, --print                                       Print response and exit
  -r, --resume [value]                              Resume a conversation by session path or latest
  --system-prompt <prompt>                          System prompt override
  -v, --version                                     Output the version number

Commands:
  agents [options]                                  List configured agents
  config [show ...]                                 Inspect discovered config files
  doctor                                            Diagnose setup, profile, and environment health
  mcp                                               Configure and manage MCP servers
  plugin|plugins                                    Manage saicode plugins
  profile [list|show [name]]                        Inspect provider profiles
  sandbox                                           Show sandbox status
  skills [list|install <path>|help]                 List or install skills
  status                                            Show current model/profile/workspace status
"#;

#[derive(Clone, Debug, Eq, PartialEq)]
enum OutputFormat {
    Text,
    Json,
    StreamJson,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ResumeTarget {
    Latest,
    Path(PathBuf),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ProcessCommand {
    Agents(Option<String>),
    Config(Option<String>),
    Doctor,
    UiBridge,
    Repl,
    Mcp(Option<String>),
    Plugins(Option<String>, Option<String>),
    Profile(Option<String>, Option<String>),
    Sandbox,
    Skills(Option<String>),
    Status,
    Tui,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CliArgs {
    print: bool,
    prompt: Option<String>,
    model: Option<String>,
    effort: Option<api::ReasoningEffort>,
    permission_mode: Option<PermissionMode>,
    allowed_tools: Vec<String>,
    disallowed_tools: Vec<String>,
    system_prompt: Option<String>,
    append_system_prompt: Option<String>,
    output_format: OutputFormat,
    resume: Option<ResumeTarget>,
    no_session_persistence: bool,
    bare: bool,
    process_command: Option<ProcessCommand>,
}

#[derive(Clone, Debug)]
struct RuntimeSurface {
    cwd: PathBuf,
    config_home: PathBuf,
    runtime_config: RuntimeConfig,
    system_prompt: Vec<String>,
    profile_name: String,
    provider_label: String,
    base_url: String,
    api_key: String,
    request_timeout_ms: u64,
    max_retries: u32,
    model: String,
    wire_model: String,
    provider_api: WireApi,
}

#[derive(Clone, Debug)]
struct ToolSurface {
    registry: GlobalToolRegistry,
    display_definitions: Vec<ToolDefinition>,
    display_to_canonical: BTreeMap<String, String>,
    permission_requirements: BTreeMap<String, PermissionMode>,
    #[cfg(test)]
    mcp_tools: BTreeMap<String, ManagedMcpTool>,
}

struct FrontlineApiClient<'a> {
    runtime: tokio::runtime::Runtime,
    client: OpenAiCompatClient,
    model: String,
    effort: Option<api::ReasoningEffort>,
    tool_definitions: Vec<ToolDefinition>,
    event_observer: Option<&'a mut dyn FnMut(&AssistantEvent)>,
    status_observer: Option<&'a mut dyn FnMut(&str)>,
}

struct FrontlineToolExecutor {
    registry: GlobalToolRegistry,
    display_to_canonical: BTreeMap<String, String>,
    permission_enforcer: PermissionEnforcer,
    runtime_config: RuntimeConfig,
    event_emitter: Option<BridgeEmitter>,
}

struct CliPermissionPrompter;

#[derive(Clone)]
struct BridgeEmitter {
    stdout: Arc<Mutex<io::BufWriter<io::Stdout>>>,
}

enum BridgeInput {
    UiReady,
    UserTurn(String),
    SlashCommand(String),
    PermissionResponse {
        decision: String,
        updated_input: Option<String>,
        reason: Option<String>,
    },
    Shutdown,
    ParseError(String),
}

struct BridgePermissionPrompter<'a> {
    receiver: &'a Receiver<BridgeInput>,
    emitter: BridgeEmitter,
}

struct BridgeSlashOutcome {
    output: String,
    should_exit: bool,
    session_changed: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(env::args().skip(1).collect())?;
    if args.process_command.is_none() && !args.print {
        if let Some(prompt) = args.prompt.as_deref() {
            if matches!(prompt, "--help" | "-h") {
                print!("{HELP_TEXT}");
                return Ok(());
            }
        }
    }

    let cwd = env::current_dir().map_err(|error| error.to_string())?;
    match &args.process_command {
        Some(command) => run_process_command(command, &cwd, &args),
        None if args.print => run_print_mode(&args, &cwd),
        None if io::stdin().is_terminal() && io::stdout().is_terminal() => tui::run(&args, &cwd),
        None => run_interactive_mode(&args, &cwd),
    }
}

fn parse_args(raw_args: Vec<String>) -> Result<CliArgs, String> {
    if raw_args.len() == 1 && raw_args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print!("{HELP_TEXT}");
        std::process::exit(0);
    }
    if raw_args.len() == 1
        && raw_args
            .iter()
            .any(|arg| arg == "--version" || arg == "-v" || arg == "-V")
    {
        println!("{VERSION_TEXT}");
        std::process::exit(0);
    }

    let mut print = false;
    let mut prompt_parts = Vec::new();
    let mut model = None;
    let mut effort = None;
    let mut permission_mode = None;
    let mut allowed_tools = Vec::new();
    let mut disallowed_tools = Vec::new();
    let mut system_prompt = None;
    let mut append_system_prompt = None;
    let mut output_format = OutputFormat::Text;
    let mut resume = None;
    let mut no_session_persistence = false;
    let mut bare = false;
    let mut process_command = None;

    let mut index = 0;
    while index < raw_args.len() {
        let arg = raw_args[index].as_str();
        if process_command.is_none() && !arg.starts_with('-') {
            process_command = parse_process_command(&raw_args[index..])?;
            if process_command.is_some() {
                break;
            }
        }
        match arg {
            "--" => {
                prompt_parts.extend(raw_args[index + 1..].iter().cloned());
                break;
            }
            "-p" | "--print" => print = true,
            "--model" => {
                index += 1;
                model = Some(required_arg(&raw_args, index, "--model")?.to_string());
            }
            "--effort" => {
                index += 1;
                effort = Some(parse_effort(required_arg(&raw_args, index, "--effort")?)?);
            }
            "--permission-mode" => {
                index += 1;
                permission_mode = Some(parse_permission_mode(required_arg(
                    &raw_args,
                    index,
                    "--permission-mode",
                )?)?);
            }
            "--dangerously-skip-permissions" => {
                permission_mode = Some(PermissionMode::DangerFullAccess);
            }
            "--allowedTools" | "--allowed-tools" | "--tools" => {
                let (values, next) = collect_tool_option_values(&raw_args, index);
                allowed_tools.extend(values);
                index = next;
            }
            "--disallowedTools" | "--disallowed-tools" => {
                let (values, next) = collect_tool_option_values(&raw_args, index);
                disallowed_tools.extend(values);
                index = next;
            }
            "--system-prompt" => {
                index += 1;
                system_prompt =
                    Some(required_arg(&raw_args, index, "--system-prompt")?.to_string());
            }
            "--system-prompt-file" => {
                index += 1;
                system_prompt = Some(
                    fs::read_to_string(required_arg(&raw_args, index, "--system-prompt-file")?)
                        .map_err(|error| format!("failed to read --system-prompt-file: {error}"))?,
                );
            }
            "--append-system-prompt" => {
                index += 1;
                append_system_prompt =
                    Some(required_arg(&raw_args, index, "--append-system-prompt")?.to_string());
            }
            "--append-system-prompt-file" => {
                index += 1;
                append_system_prompt = Some(
                    fs::read_to_string(required_arg(
                        &raw_args,
                        index,
                        "--append-system-prompt-file",
                    )?)
                    .map_err(|error| {
                        format!("failed to read --append-system-prompt-file: {error}")
                    })?,
                );
            }
            "--output-format" => {
                index += 1;
                output_format =
                    parse_output_format(required_arg(&raw_args, index, "--output-format")?)?;
            }
            "-r" | "--resume" => {
                resume = Some(parse_resume_target(&raw_args, &mut index));
            }
            "-c" | "--continue" => {
                resume = Some(ResumeTarget::Latest);
            }
            "--no-session-persistence" => no_session_persistence = true,
            "--bare" => bare = true,
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}\n\n{HELP_TEXT}"));
            }
            value => prompt_parts.push(value.to_string()),
        }
        index += 1;
    }

    let prompt = if prompt_parts.is_empty() {
        None
    } else {
        Some(prompt_parts.join(" ").trim().to_string())
    };

    Ok(CliArgs {
        print,
        prompt,
        model,
        effort,
        permission_mode,
        allowed_tools,
        disallowed_tools,
        system_prompt,
        append_system_prompt,
        output_format,
        resume,
        no_session_persistence,
        bare,
        process_command,
    })
}

fn parse_process_command(args: &[String]) -> Result<Option<ProcessCommand>, String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(None);
    };

    let remainder = args
        .get(1..)
        .unwrap_or(&[])
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    match command {
        "agents" => Ok(Some(ProcessCommand::Agents(join_optional_args(&remainder)))),
        "config" => Ok(Some(ProcessCommand::Config(join_optional_args(&remainder)))),
        "doctor" => Ok(Some(ProcessCommand::Doctor)),
        "ui-bridge" => Ok(Some(ProcessCommand::UiBridge)),
        "repl" => Ok(Some(ProcessCommand::Repl)),
        "mcp" => Ok(Some(ProcessCommand::Mcp(join_optional_args(&remainder)))),
        "plugin" | "plugins" => {
            let action = remainder.first().map(|value| (*value).to_string());
            let target = if remainder.len() > 1 {
                Some(remainder[1..].join(" "))
            } else {
                None
            };
            Ok(Some(ProcessCommand::Plugins(action, target)))
        }
        "profile" => {
            let action = remainder.first().map(|value| (*value).to_string());
            let target = remainder.get(1).map(|value| (*value).to_string());
            Ok(Some(ProcessCommand::Profile(action, target)))
        }
        "sandbox" => Ok(Some(ProcessCommand::Sandbox)),
        "skills" => Ok(Some(ProcessCommand::Skills(join_optional_args(&remainder)))),
        "status" => Ok(Some(ProcessCommand::Status)),
        "tui" => Ok(Some(ProcessCommand::Tui)),
        _ => Ok(None),
    }
}

fn required_arg<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn collect_tool_option_values(args: &[String], start_index: usize) -> (Vec<String>, usize) {
    let mut values = Vec::new();
    let mut index = start_index + 1;
    while index < args.len() {
        let value = args[index].trim();
        if value.is_empty() {
            index += 1;
            continue;
        }
        if value == "--" || value.starts_with('-') {
            break;
        }
        if !looks_like_tool_option_value(value) {
            break;
        }
        values.push(args[index].clone());
        index += 1;
    }
    let next_index = if values.is_empty() {
        start_index
    } else {
        index - 1
    };
    (values, next_index)
}

fn looks_like_tool_option_value(value: &str) -> bool {
    let selector_names = GlobalToolRegistry::builtin().selector_names();
    value
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter(|token| !token.is_empty())
        .all(|token| {
            let normalized = token.split('(').next().unwrap_or(token);
            selector_names
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(normalized))
        })
}

fn parse_resume_target(args: &[String], index: &mut usize) -> ResumeTarget {
    if let Some(candidate) = args.get(*index + 1) {
        if !candidate.starts_with('-') {
            *index += 1;
            return ResumeTarget::Path(PathBuf::from(candidate));
        }
    }
    ResumeTarget::Latest
}

fn parse_effort(value: &str) -> Result<api::ReasoningEffort, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "low" => Ok(api::ReasoningEffort::Low),
        "medium" => Ok(api::ReasoningEffort::Medium),
        "high" => Ok(api::ReasoningEffort::High),
        "max" | "xhigh" => Ok(api::ReasoningEffort::Max),
        _ => Err("--effort must be one of: low, medium, high, max, xhigh".to_string()),
    }
}

fn parse_permission_mode(value: &str) -> Result<PermissionMode, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "read-only" => Ok(PermissionMode::ReadOnly),
        "workspace-write" => Ok(PermissionMode::WorkspaceWrite),
        "danger-full-access" => Ok(PermissionMode::DangerFullAccess),
        _ => Err(
            "--permission-mode must be one of: read-only, workspace-write, danger-full-access"
                .to_string(),
        ),
    }
}

fn parse_output_format(value: &str) -> Result<OutputFormat, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        "stream-json" => Ok(OutputFormat::StreamJson),
        _ => Err("--output-format must be one of: text, json, stream-json".to_string()),
    }
}

fn run_process_command(command: &ProcessCommand, cwd: &Path, args: &CliArgs) -> Result<(), String> {
    if matches!(command, ProcessCommand::UiBridge) {
        return run_ui_bridge(args, cwd);
    }
    if matches!(command, ProcessCommand::Tui) {
        return tui::run(args, cwd);
    }
    let surface = load_runtime_surface(cwd, args)?;
    match command {
        ProcessCommand::Agents(raw) => {
            println!(
                "{}",
                handle_agents_slash_command(raw.as_deref(), cwd)
                    .map_err(|error| error.to_string())?
            );
        }
        ProcessCommand::Config(raw) => {
            println!(
                "{}",
                render_config_report(&surface.runtime_config, raw.as_deref())
            );
        }
        ProcessCommand::Doctor => {
            println!(
                "{}",
                render_doctor_report(
                    &surface,
                    resolved_permission_mode(args, &surface.runtime_config),
                )?
            );
        }
        ProcessCommand::UiBridge => unreachable!("ui bridge handled before loading surface"),
        ProcessCommand::Repl => {
            return run_interactive_mode(args, cwd);
        }
        ProcessCommand::Mcp(raw) => {
            println!(
                "{}",
                handle_mcp_slash_command(raw.as_deref(), cwd).map_err(|error| error.to_string())?
            );
        }
        ProcessCommand::Plugins(action, target) => {
            let mut manager = build_plugin_manager(&surface.runtime_config, &surface.config_home);
            let result =
                handle_plugins_slash_command(action.as_deref(), target.as_deref(), &mut manager)
                    .map_err(|error| error.to_string())?;
            println!("{}", result.message);
        }
        ProcessCommand::Profile(action, target) => {
            println!(
                "{}",
                render_profile_report(
                    &surface.runtime_config,
                    action.as_deref(),
                    target.as_deref()
                )?
            );
        }
        ProcessCommand::Sandbox => {
            println!("{}", render_sandbox_report(&surface));
        }
        ProcessCommand::Skills(raw) => {
            println!(
                "{}",
                handle_skills_slash_command(raw.as_deref(), cwd)
                    .map_err(|error| error.to_string())?
            );
        }
        ProcessCommand::Status => {
            println!(
                "{}",
                render_status_report(
                    &surface,
                    resolved_permission_mode(args, &surface.runtime_config),
                    None,
                )
            );
        }
        ProcessCommand::Tui => unreachable!("tui handled before loading surface"),
    }
    Ok(())
}

fn run_print_mode(args: &CliArgs, cwd: &Path) -> Result<(), String> {
    let prompt = resolve_prompt_input(args.prompt.clone())?;
    if prompt.trim().is_empty() {
        return Err("prompt is required".to_string());
    }

    let surface = load_runtime_surface(cwd, args)?;
    let tool_surface = build_tool_surface(args, &surface.runtime_config, &surface.config_home)?;
    let mut session = load_or_create_session(args, &surface.config_home)?;
    let mut prompter = io::stdin().is_terminal().then_some(CliPermissionPrompter);
    let mut stdout = io::stdout();
    let mut text_renderer = InteractiveRenderer::default();
    let mut text_emitter = |event: &AssistantEvent| text_renderer.observe(event);
    let mut stream_emitter = |event: &AssistantEvent| emit_stream_json_event(&mut stdout, event);
    let summary = run_conversation_turn(
        &mut session,
        &surface,
        &tool_surface,
        args.effort,
        resolved_permission_mode(args, &surface.runtime_config),
        &prompt,
        prompter
            .as_mut()
            .map(|value| value as &mut dyn PermissionPrompter),
        match args.output_format {
            OutputFormat::Text => Some(&mut text_emitter as &mut dyn FnMut(&AssistantEvent)),
            OutputFormat::StreamJson => {
                Some(&mut stream_emitter as &mut dyn FnMut(&AssistantEvent))
            }
            OutputFormat::Json => None,
        },
        None,
        None,
    )?;
    let text = final_assistant_text(&summary);
    match args.output_format {
        OutputFormat::Text => text_renderer.finish(&summary)?,
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "text": text,
                    "model": surface.model,
                    "usage": {
                        "input_tokens": summary.usage.input_tokens,
                        "output_tokens": summary.usage.output_tokens,
                        "cache_creation_input_tokens": summary.usage.cache_creation_input_tokens,
                        "cache_read_input_tokens": summary.usage.cache_read_input_tokens,
                    },
                    "session": {
                        "id": session.session_id,
                        "path": session.persistence_path().map(|path| path.display().to_string()),
                    }
                }))
                .map_err(|error| error.to_string())?
            );
        }
        OutputFormat::StreamJson => {
            emit_stream_json_line(
                &mut stdout,
                json!({
                    "type": "final_message",
                    "text": text,
                    "model": surface.model,
                    "usage": {
                        "input_tokens": summary.usage.input_tokens,
                        "output_tokens": summary.usage.output_tokens,
                        "cache_creation_input_tokens": summary.usage.cache_creation_input_tokens,
                        "cache_read_input_tokens": summary.usage.cache_read_input_tokens,
                    },
                    "session": {
                        "id": session.session_id,
                        "path": session.persistence_path().map(|path| path.display().to_string()),
                    }
                }),
            )?;
        }
    }
    Ok(())
}

fn run_interactive_mode(args: &CliArgs, cwd: &Path) -> Result<(), String> {
    let mut surface = load_runtime_surface(cwd, args)?;
    let tool_surface = build_tool_surface(args, &surface.runtime_config, &surface.config_home)?;
    let mut session = load_or_create_session(args, &surface.config_home)?;
    let mut model = surface.model.clone();
    let mut effort = args.effort;
    let mut permission_mode = resolved_permission_mode(args, &surface.runtime_config);
    let mut stdout = io::stdout();
    writeln!(
        stdout,
        "saicode Rust frontline  model={model}  permission={}  /help for commands",
        permission_mode.as_str()
    )
    .map_err(|error| error.to_string())?;
    write!(stdout, "saicode> ").map_err(|error| error.to_string())?;
    stdout.flush().map_err(|error| error.to_string())?;

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    while let Some(line) = read_next_line(&mut lines)? {
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input.starts_with('/') {
            let previous_model = model.clone();
            if handle_interactive_slash_command(
                input,
                &surface,
                &tool_surface,
                &mut session,
                &mut model,
                &mut effort,
                &mut permission_mode,
            )? {
                break;
            }
            if model != previous_model {
                match load_runtime_surface_for_model(cwd, args, &model) {
                    Ok(updated_surface) => {
                        model = updated_surface.model.clone();
                        surface = updated_surface;
                    }
                    Err(error) => {
                        model = previous_model;
                        writeln!(stdout, "{error}").map_err(|io_error| io_error.to_string())?;
                    }
                }
            }
            print!("saicode> ");
            io::stdout().flush().map_err(|error| error.to_string())?;
            continue;
        }

        let mut interactive_renderer = InteractiveRenderer::default();
        let mut prompter = CliPermissionPrompter;
        let summary = run_conversation_turn(
            &mut session,
            &surface,
            &tool_surface,
            effort,
            permission_mode,
            input,
            Some(&mut prompter),
            Some(&mut |event| interactive_renderer.observe(event)),
            None,
            None,
        )?;
        interactive_renderer.finish(&summary)?;
        print!("saicode> ");
        io::stdout().flush().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn run_ui_bridge(args: &CliArgs, cwd: &Path) -> Result<(), String> {
    let mut surface = load_runtime_surface(cwd, args)?;
    let tool_surface = build_tool_surface(args, &surface.runtime_config, &surface.config_home)?;
    let emitter = BridgeEmitter::new();
    let (sender, receiver) = mpsc::channel::<BridgeInput>();
    spawn_bridge_stdin_reader(sender);

    let mut session = load_or_create_session(args, &surface.config_home)?;
    let mut model = surface.model.clone();
    let mut effort = args.effort;
    let mut permission_mode = resolved_permission_mode(args, &surface.runtime_config);
    emitter.emit(bridge_session_payload(
        "session_started",
        &surface,
        &tool_surface,
        &session,
        permission_mode,
    ))?;

    while let Ok(message) = receiver.recv() {
        match message {
            BridgeInput::UiReady => {
                emitter.emit(json!({ "type": "ready" }))?;
            }
            BridgeInput::UserTurn(prompt) => {
                let trimmed = prompt.trim();
                if trimmed.is_empty() {
                    continue;
                }
                emitter.emit(json!({ "type": "turn_started" }))?;
                emit_bridge_status(&emitter, "Preparing request…");
                let event_emitter = emitter.clone();
                let status_emitter = emitter.clone();
                let mut prompter = BridgePermissionPrompter {
                    receiver: &receiver,
                    emitter: emitter.clone(),
                };
                match run_conversation_turn(
                    &mut session,
                    &surface,
                    &tool_surface,
                    effort,
                    permission_mode,
                    trimmed,
                    Some(&mut prompter),
                    Some(&mut |event| emit_bridge_assistant_event(&event_emitter, event)),
                    Some(&mut |status| emit_bridge_status(&status_emitter, status)),
                    Some(emitter.clone()),
                ) {
                    Ok(summary) => {
                        let text = final_assistant_text(&summary);
                        emitter.emit(json!({
                            "type": "final_message",
                            "text": text,
                            "model": model,
                            "usage": {
                                "input_tokens": summary.usage.input_tokens,
                                "output_tokens": summary.usage.output_tokens,
                                "cache_creation_input_tokens": summary.usage.cache_creation_input_tokens,
                                "cache_read_input_tokens": summary.usage.cache_read_input_tokens,
                            },
                            "session": {
                                "id": session.session_id,
                                "path": session.persistence_path().map(|path| path.display().to_string()),
                            }
                        }))?;
                        if let Some(auto_compaction) = summary.auto_compaction {
                            emitter.emit(json!({
                                "type": "auto_compaction",
                                "removed_message_count": auto_compaction.removed_message_count,
                            }))?;
                        }
                    }
                    Err(error) => {
                        emit_bridge_status(&emitter, "Request failed.");
                        emitter.emit(json!({
                            "type": "error",
                            "message": error,
                        }))?;
                    }
                }
                emitter.emit(bridge_session_payload(
                    "session_updated",
                    &surface,
                    &tool_surface,
                    &session,
                    permission_mode,
                ))?;
            }
            BridgeInput::SlashCommand(input) => {
                let previous_model = model.clone();
                let outcome = handle_bridge_slash_command(
                    &input,
                    &surface,
                    &mut session,
                    &mut model,
                    &mut effort,
                    &mut permission_mode,
                )?;
                let mut surface_changed = false;
                if model != previous_model {
                    match load_runtime_surface_for_model(cwd, args, &model) {
                        Ok(updated_surface) => {
                            model = updated_surface.model.clone();
                            surface = updated_surface;
                            surface_changed = true;
                        }
                        Err(error) => {
                            model = previous_model;
                            emitter.emit(json!({
                                "type": "error",
                                "message": error,
                            }))?;
                        }
                    }
                }
                emitter.emit(json!({
                    "type": "slash_result",
                    "input": input,
                    "text": outcome.output,
                    "should_exit": outcome.should_exit,
                }))?;
                if outcome.session_changed || surface_changed {
                    emitter.emit(bridge_session_payload(
                        "session_updated",
                        &surface,
                        &tool_surface,
                        &session,
                        permission_mode,
                    ))?;
                }
                if outcome.should_exit {
                    break;
                }
            }
            BridgeInput::PermissionResponse { .. } => {
                emitter.emit(json!({
                    "type": "error",
                    "message": "permission_response arrived without an active permission request",
                }))?;
            }
            BridgeInput::Shutdown => break,
            BridgeInput::ParseError(message) => {
                emitter.emit(json!({ "type": "error", "message": message }))?;
            }
        }
    }

    emitter.emit(json!({ "type": "shutdown_complete" }))?;
    Ok(())
}

fn handle_interactive_slash_command(
    input: &str,
    surface: &RuntimeSurface,
    _tool_surface: &ToolSurface,
    session: &mut Session,
    model: &mut String,
    effort: &mut Option<api::ReasoningEffort>,
    permission_mode: &mut PermissionMode,
) -> Result<bool, String> {
    let command = match validate_slash_command_input(input) {
        Ok(Some(command)) => command,
        Ok(None) => return Ok(false),
        Err(error) => {
            println!("{error}");
            return Ok(false);
        }
    };

    match command {
        SlashCommand::Help => println!("{}", render_slash_command_help()),
        SlashCommand::Compact => {
            let result = compact_session(session, runtime::CompactionConfig::default());
            *session = result.compacted_session;
            if result.removed_message_count == 0 {
                println!("Compaction skipped: session is below the threshold.");
            } else {
                println!(
                    "Compacted {} messages into a resumable summary.",
                    result.removed_message_count
                );
            }
        }
        SlashCommand::Status => println!(
            "{}",
            render_status_report(surface, *permission_mode, session.persistence_path())
        ),
        SlashCommand::Model { model: next_model } => {
            if let Some(next_model) = next_model {
                *model = next_model;
                println!("model -> {}", model);
            } else {
                println!("model = {}", model);
            }
        }
        SlashCommand::Effort { level } => {
            if let Some(level) = level {
                *effort = Some(parse_effort(&level)?);
                println!(
                    "effort -> {}",
                    effort
                        .map(|value| value.saicode_wire_value())
                        .unwrap_or("default")
                );
            } else {
                println!(
                    "effort = {}",
                    effort
                        .map(|value| value.saicode_wire_value())
                        .unwrap_or("default")
                );
            }
        }
        SlashCommand::Permissions { mode } => {
            if let Some(mode) = mode {
                *permission_mode = parse_permission_mode(&mode)?;
                println!("permission-mode -> {}", permission_mode.as_str());
            } else {
                println!("permission-mode = {}", permission_mode.as_str());
            }
        }
        SlashCommand::Cost => {
            for line in UsageTracker::from_session(session)
                .cumulative_usage()
                .summary_lines_for_model("usage", Some(model))
            {
                println!("{line}");
            }
        }
        SlashCommand::Clear { .. } => {
            *session = Session::new().with_persistence_path(new_session_path(&surface.config_home));
            println!("Started a fresh session: {}", session.session_id);
        }
        SlashCommand::Resume { session_path } => {
            let target = session_path
                .map(PathBuf::from)
                .or_else(|| latest_session_path(&surface.config_home))
                .ok_or_else(|| "no resumable session found".to_string())?;
            *session = Session::load_from_path(&target).map_err(|error| error.to_string())?;
            println!("Loaded session {}", target.display());
        }
        SlashCommand::Version => println!("{VERSION_TEXT}"),
        SlashCommand::Exit => return Ok(true),
        SlashCommand::Config { section } => {
            println!(
                "{}",
                render_config_report(&surface.runtime_config, section.as_deref())
            );
        }
        SlashCommand::Doctor => println!("{}", render_doctor_report(surface, *permission_mode)?),
        SlashCommand::Sandbox => {
            println!("{}", render_sandbox_report(surface));
        }
        SlashCommand::Mcp { action, target } => {
            let args = match (action, target) {
                (Some(action), Some(target)) => Some(format!("{action} {target}")),
                (Some(action), None) => Some(action),
                (None, Some(target)) => Some(target),
                (None, None) => None,
            };
            println!(
                "{}",
                handle_mcp_slash_command(args.as_deref(), &surface.cwd)
                    .map_err(|error| error.to_string())?
            );
        }
        SlashCommand::Agents { args } => {
            println!(
                "{}",
                handle_agents_slash_command(args.as_deref(), &surface.cwd)
                    .map_err(|error| error.to_string())?
            );
        }
        SlashCommand::Skills { args } => {
            println!(
                "{}",
                handle_skills_slash_command(args.as_deref(), &surface.cwd)
                    .map_err(|error| error.to_string())?
            );
        }
        SlashCommand::Plugins { action, target } => {
            let mut manager = build_plugin_manager(&surface.runtime_config, &surface.config_home);
            let result: PluginsCommandResult =
                handle_plugins_slash_command(action.as_deref(), target.as_deref(), &mut manager)
                    .map_err(|error| error.to_string())?;
            println!("{}", result.message);
        }
        SlashCommand::Memory => println!("{}", render_memory_report()?),
        _ => println!(
            "{}",
            unsupported_slash_command_message(input, "Rust frontline")
        ),
    }

    Ok(false)
}

fn handle_bridge_slash_command(
    input: &str,
    surface: &RuntimeSurface,
    session: &mut Session,
    model: &mut String,
    effort: &mut Option<api::ReasoningEffort>,
    permission_mode: &mut PermissionMode,
) -> Result<BridgeSlashOutcome, String> {
    let command = match validate_slash_command_input(input) {
        Ok(Some(command)) => command,
        Ok(None) => {
            return Ok(BridgeSlashOutcome {
                output: String::new(),
                should_exit: false,
                session_changed: false,
            })
        }
        Err(error) => {
            return Ok(BridgeSlashOutcome {
                output: error.to_string(),
                should_exit: false,
                session_changed: false,
            })
        }
    };

    let mut should_exit = false;
    let mut session_changed = false;

    let output = match command {
        SlashCommand::Help => render_slash_command_help(),
        SlashCommand::Compact => {
            let result = compact_session(session, runtime::CompactionConfig::default());
            *session = result.compacted_session;
            session_changed = true;
            if result.removed_message_count == 0 {
                "Compaction skipped: session is below the threshold.".to_string()
            } else {
                format!(
                    "Compacted {} messages into a resumable summary.",
                    result.removed_message_count
                )
            }
        }
        SlashCommand::Status => {
            render_status_report(surface, *permission_mode, session.persistence_path())
        }
        SlashCommand::Model { model: next_model } => {
            if let Some(next_model) = next_model {
                *model = next_model;
                format!("model -> {}", model)
            } else {
                format!("model = {}", model)
            }
        }
        SlashCommand::Effort { level } => {
            if let Some(level) = level {
                *effort = Some(parse_effort(&level)?);
                format!(
                    "effort -> {}",
                    effort
                        .map(|value| value.saicode_wire_value())
                        .unwrap_or("default")
                )
            } else {
                format!(
                    "effort = {}",
                    effort
                        .map(|value| value.saicode_wire_value())
                        .unwrap_or("default")
                )
            }
        }
        SlashCommand::Permissions { mode } => {
            if let Some(mode) = mode {
                *permission_mode = parse_permission_mode(&mode)?;
                session_changed = true;
                format!("permission-mode -> {}", permission_mode.as_str())
            } else {
                format!("permission-mode = {}", permission_mode.as_str())
            }
        }
        SlashCommand::Cost => UsageTracker::from_session(session)
            .cumulative_usage()
            .summary_lines_for_model("usage", Some(model))
            .join("\n"),
        SlashCommand::Clear { .. } => {
            *session = Session::new().with_persistence_path(new_session_path(&surface.config_home));
            session_changed = true;
            format!("Started a fresh session: {}", session.session_id)
        }
        SlashCommand::Resume { session_path } => {
            let target = session_path
                .map(PathBuf::from)
                .or_else(|| latest_session_path(&surface.config_home))
                .ok_or_else(|| "no resumable session found".to_string())?;
            *session = Session::load_from_path(&target).map_err(|error| error.to_string())?;
            session_changed = true;
            format!("Loaded session {}", target.display())
        }
        SlashCommand::Version => VERSION_TEXT.to_string(),
        SlashCommand::Exit => {
            should_exit = true;
            "Exiting session.".to_string()
        }
        SlashCommand::Config { section } => {
            render_config_report(&surface.runtime_config, section.as_deref())
        }
        SlashCommand::Doctor => render_doctor_report(surface, *permission_mode)?,
        SlashCommand::Sandbox => render_sandbox_report(surface),
        SlashCommand::Mcp { action, target } => {
            let args = match (action, target) {
                (Some(action), Some(target)) => Some(format!("{action} {target}")),
                (Some(action), None) => Some(action),
                (None, Some(target)) => Some(target),
                (None, None) => None,
            };
            handle_mcp_slash_command(args.as_deref(), &surface.cwd)
                .map_err(|error| error.to_string())?
        }
        SlashCommand::Agents { args } => handle_agents_slash_command(args.as_deref(), &surface.cwd)
            .map_err(|error| error.to_string())?,
        SlashCommand::Skills { args } => handle_skills_slash_command(args.as_deref(), &surface.cwd)
            .map_err(|error| error.to_string())?,
        SlashCommand::Plugins { action, target } => {
            let mut manager = build_plugin_manager(&surface.runtime_config, &surface.config_home);
            let result: PluginsCommandResult =
                handle_plugins_slash_command(action.as_deref(), target.as_deref(), &mut manager)
                    .map_err(|error| error.to_string())?;
            result.message
        }
        SlashCommand::Memory => render_memory_report()?,
        _ => unsupported_slash_command_message(input, "Rust backend"),
    };

    Ok(BridgeSlashOutcome {
        output,
        should_exit,
        session_changed,
    })
}

fn run_conversation_turn<'a>(
    session: &mut Session,
    surface: &RuntimeSurface,
    tool_surface: &ToolSurface,
    effort: Option<api::ReasoningEffort>,
    permission_mode: PermissionMode,
    prompt: &str,
    prompter: Option<&mut dyn PermissionPrompter>,
    on_event: Option<&'a mut dyn FnMut(&AssistantEvent)>,
    status_observer: Option<&'a mut dyn FnMut(&str)>,
    tool_event_emitter: Option<BridgeEmitter>,
) -> Result<runtime::TurnSummary, String> {
    let mut runtime = ConversationRuntime::new_with_features(
        session.clone(),
        FrontlineApiClient::new(surface, tool_surface, effort, on_event, status_observer)?,
        FrontlineToolExecutor {
            registry: tool_surface.registry.clone(),
            display_to_canonical: tool_surface.display_to_canonical.clone(),
            permission_enforcer: PermissionEnforcer::new(build_permission_policy(
                tool_surface,
                permission_mode,
                &surface.runtime_config,
            )),
            runtime_config: surface.runtime_config.clone(),
            event_emitter: tool_event_emitter,
        },
        build_permission_policy(tool_surface, permission_mode, &surface.runtime_config),
        surface.system_prompt.clone(),
        surface.runtime_config.feature_config(),
    );
    let summary = runtime
        .run_turn(prompt, prompter)
        .map_err(|error| error.to_string())?;
    *session = runtime.into_session();
    Ok(summary)
}

fn load_runtime_surface(cwd: &Path, args: &CliArgs) -> Result<RuntimeSurface, String> {
    let loader = runtime::ConfigLoader::default_for(cwd);
    let runtime_config = loader.load().map_err(|error| error.to_string())?;
    let resolved = ProfileResolver::resolve(&runtime_config, None, args.model.as_deref())
        .or_else(|_| ProfileResolver::resolve(&runtime_config, None, Some(DEFAULT_MODEL)))
        .map_err(|error| error.to_string())?;
    let launch = ProviderLauncher::prepare(&resolved).map_err(|error| error.to_string())?;
    let resolved_model = resolve_model(Some(resolved.model.as_str()));
    let provider = get_provider_config(&resolved_model)?;
    let api_key = provider
        .api_key
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("provider `{}` does not have credentials", provider.id))?;
    let mut system_prompt = if args.bare {
        load_system_prompt_bare(
            cwd.to_path_buf(),
            current_date_string(),
            env::consts::OS,
            env::consts::ARCH,
        )
    } else {
        load_system_prompt(
            cwd.to_path_buf(),
            current_date_string(),
            env::consts::OS,
            env::consts::ARCH,
        )
    }
    .map_err(|error| error.to_string())?;
    if let Some(base) = &args.system_prompt {
        system_prompt = vec![base.clone()];
    }
    if let Some(extra) = &args.append_system_prompt {
        system_prompt.push(extra.clone());
    }
    system_prompt.push(GlobalToolRegistry::builtin().prompt_guidance());
    Ok(RuntimeSurface {
        cwd: cwd.to_path_buf(),
        config_home: loader.config_home().to_path_buf(),
        runtime_config,
        system_prompt,
        profile_name: resolved.profile_name,
        provider_label: provider.id.clone(),
        base_url: provider.base_url.clone(),
        api_key,
        request_timeout_ms: launch.request_timeout_ms,
        max_retries: launch.max_retries,
        model: resolved_model.alias,
        wire_model: resolved_model.model,
        provider_api: provider.api,
    })
}

fn load_runtime_surface_for_model(
    cwd: &Path,
    args: &CliArgs,
    model: &str,
) -> Result<RuntimeSurface, String> {
    let mut next_args = args.clone();
    next_args.model = Some(model.to_string());
    load_runtime_surface(cwd, &next_args)
}

fn build_tool_surface(
    args: &CliArgs,
    runtime_config: &RuntimeConfig,
    config_home: &Path,
) -> Result<ToolSurface, String> {
    let plugin_tools = build_plugin_manager(runtime_config, config_home)
        .aggregated_tools()
        .map_err(|error| error.to_string())?;
    let registry = if plugin_tools.is_empty() {
        GlobalToolRegistry::builtin()
    } else {
        GlobalToolRegistry::with_plugin_tools(plugin_tools)?
    };
    let mcp_tools = discover_mcp_tools(runtime_config)?;
    let allowed = normalize_tool_selection(&registry, &mcp_tools, &args.allowed_tools)?;
    let denied = normalize_tool_selection(&registry, &mcp_tools, &args.disallowed_tools)?;
    let builtin_plugin_allowed = builtin_plugin_allowlist(allowed.as_ref());
    let mut allowed_names = registry
        .definitions(builtin_plugin_allowed.as_ref())
        .into_iter()
        .map(|definition| definition.name)
        .collect::<BTreeSet<_>>();
    for tool in &mcp_tools {
        if allowed
            .as_ref()
            .is_none_or(|selected| selected.contains(&tool.qualified_name))
        {
            allowed_names.insert(tool.qualified_name.clone());
        }
    }
    if let Some(denied) = denied {
        for item in denied {
            allowed_names.remove(&item);
        }
    }

    let builtin_plugin_effective = builtin_plugin_allowlist(Some(&allowed_names));
    let definitions = registry.definitions(builtin_plugin_effective.as_ref());
    let permission_specs = registry.permission_specs(builtin_plugin_effective.as_ref())?;
    let mut display_definitions = Vec::new();
    let mut display_to_canonical = BTreeMap::new();
    let mut permission_requirements = BTreeMap::new();
    #[cfg(test)]
    let mut mcp_tool_map = BTreeMap::new();

    for definition in definitions {
        let display_name = registry_display_name(&definition.name);
        display_to_canonical.insert(display_name.clone(), definition.name.clone());
        permission_requirements.insert(display_name.clone(), PermissionMode::ReadOnly);
        display_definitions.push(ToolDefinition {
            name: display_name,
            description: definition.description,
            input_schema: definition.input_schema,
        });
    }
    for (canonical, mode) in permission_specs {
        let display_name = registry_display_name(&canonical);
        permission_requirements.insert(display_name, mode);
    }
    for tool in mcp_tools {
        if !allowed_names.contains(&tool.qualified_name) {
            continue;
        }
        let display_name = tool.qualified_name.clone();
        display_to_canonical.insert(display_name.clone(), tool.qualified_name.clone());
        permission_requirements.insert(display_name.clone(), PermissionMode::DangerFullAccess);
        display_definitions.push(ToolDefinition {
            name: display_name.clone(),
            description: tool.tool.description.clone(),
            input_schema: tool.tool.input_schema.clone().unwrap_or_else(|| json!({})),
        });
        #[cfg(test)]
        mcp_tool_map.insert(tool.qualified_name.clone(), tool);
    }

    Ok(ToolSurface {
        registry,
        display_definitions,
        display_to_canonical,
        permission_requirements,
        #[cfg(test)]
        mcp_tools: mcp_tool_map,
    })
}

fn discover_mcp_tools(runtime_config: &RuntimeConfig) -> Result<Vec<ManagedMcpTool>, String> {
    if runtime_config.mcp().servers().is_empty() {
        return Ok(Vec::new());
    }
    let mut manager = McpServerManager::from_runtime_config(runtime_config);
    let rt = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    rt.block_on(manager.discover_tools())
        .map_err(|error| format!("failed to discover MCP tools: {error}"))
}

fn normalize_tool_selection(
    registry: &GlobalToolRegistry,
    mcp_tools: &[ManagedMcpTool],
    values: &[String],
) -> Result<Option<BTreeSet<String>>, String> {
    if values.is_empty() {
        return Ok(None);
    }
    let mut selected = BTreeSet::new();
    let mcp_name_map = mcp_name_map(mcp_tools);
    let available_names = registry
        .selector_names()
        .into_iter()
        .chain(mcp_tools.iter().map(|tool| tool.qualified_name.clone()))
        .collect::<Vec<_>>();

    for token in split_tool_selection_values(values) {
        match registry.normalize_allowed_tools(&[token.clone()]) {
            Ok(Some(names)) => {
                selected.extend(names);
                continue;
            }
            Ok(None) => continue,
            Err(_) => {}
        }
        let normalized = normalize_requested_tool_name(&token);
        if let Some(canonical) = mcp_name_map.get(&normalized) {
            selected.insert(canonical.clone());
            continue;
        }
        return Err(format!(
            "unsupported tool in --allowedTools/--disallowedTools: {token} (expected one of: {})",
            available_names.join(", ")
        ));
    }

    Ok(Some(selected))
}

fn split_tool_selection_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| {
            value
                .split(|ch: char| ch == ',' || ch.is_whitespace())
                .map(str::trim)
                .filter(|token| !token.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn mcp_name_map(mcp_tools: &[ManagedMcpTool]) -> BTreeMap<String, String> {
    let mut name_map = BTreeMap::new();
    for tool in mcp_tools {
        name_map.insert(
            normalize_requested_tool_name(&tool.qualified_name),
            tool.qualified_name.clone(),
        );
    }
    name_map
}

fn normalize_requested_tool_name(value: &str) -> String {
    value.trim().replace('-', "_").to_ascii_lowercase()
}

fn builtin_plugin_allowlist(selected: Option<&BTreeSet<String>>) -> Option<BTreeSet<String>> {
    selected.map(|items| {
        items
            .iter()
            .filter(|item| !item.starts_with("mcp__"))
            .cloned()
            .collect::<BTreeSet<_>>()
    })
}

fn build_permission_policy(
    tool_surface: &ToolSurface,
    permission_mode: PermissionMode,
    runtime_config: &RuntimeConfig,
) -> PermissionPolicy {
    let mut policy = PermissionPolicy::new(permission_mode)
        .with_permission_rules(runtime_config.permission_rules());
    for (display_name, required_mode) in &tool_surface.permission_requirements {
        policy = policy.with_tool_requirement(display_name.clone(), *required_mode);
    }
    policy
}

fn load_or_create_session(args: &CliArgs, config_home: &Path) -> Result<Session, String> {
    if let Some(resume) = &args.resume {
        let path = match resume {
            ResumeTarget::Latest => latest_session_path(config_home),
            ResumeTarget::Path(path) => Some(path.clone()),
        }
        .ok_or_else(|| "no resumable session found".to_string())?;
        return Session::load_from_path(path).map_err(|error| error.to_string());
    }

    if args.no_session_persistence {
        return Ok(Session::new());
    }

    Ok(Session::new().with_persistence_path(new_session_path(config_home)))
}

fn new_session_path(config_home: &Path) -> PathBuf {
    let sessions_dir = config_home.join("sessions");
    let _ = fs::create_dir_all(&sessions_dir);
    sessions_dir.join(format!("session-{}{}", now_millis(), SESSION_FILE_SUFFIX))
}

fn latest_session_path(config_home: &Path) -> Option<PathBuf> {
    let sessions_dir = config_home.join("sessions");
    let mut entries = fs::read_dir(sessions_dir)
        .ok()?
        .flatten()
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "jsonl")
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    entries.last().map(|entry| entry.path())
}

fn resolved_permission_mode(args: &CliArgs, config: &RuntimeConfig) -> PermissionMode {
    args.permission_mode
        .unwrap_or_else(|| match config.permission_mode() {
            Some(ResolvedPermissionMode::ReadOnly)
            | Some(ResolvedPermissionMode::WorkspaceWrite)
            | Some(ResolvedPermissionMode::DangerFullAccess)
            | None => PermissionMode::DangerFullAccess,
        })
}

fn registry_display_name(canonical: &str) -> String {
    tools::mvp_tool_specs()
        .into_iter()
        .find(|spec| spec.name == canonical)
        .map(|spec| spec.display_name().to_string())
        .unwrap_or_else(|| canonical.to_string())
}

#[derive(Default)]
struct InteractiveRenderer {
    saw_text: bool,
    line_open: bool,
}

impl InteractiveRenderer {
    fn observe(&mut self, event: &AssistantEvent) {
        match event {
            AssistantEvent::TextDelta(text) => {
                if !text.is_empty() {
                    print!("{text}");
                    let _ = io::stdout().flush();
                    self.saw_text = true;
                    self.line_open = true;
                }
            }
            AssistantEvent::ToolUse { name, .. } => {
                if self.line_open {
                    println!();
                }
                println!("[tool] {name}");
                self.line_open = false;
            }
            AssistantEvent::MessageStop
            | AssistantEvent::Usage(_)
            | AssistantEvent::PromptCache(_) => {}
        }
    }

    fn finish(&mut self, summary: &runtime::TurnSummary) -> Result<(), String> {
        if self.line_open {
            println!();
            self.line_open = false;
        }
        if !self.saw_text {
            println!("{}", final_assistant_text(summary));
        }
        Ok(())
    }
}

fn emit_stream_json_event(stdout: &mut impl Write, event: &AssistantEvent) {
    let payload = match event {
        AssistantEvent::TextDelta(text) => json!({ "type": "content_delta", "delta": text }),
        AssistantEvent::ToolUse { id, name, input } => {
            json!({ "type": "tool_start", "id": id, "name": name, "input": parse_tool_input(input) })
        }
        AssistantEvent::Usage(usage) => json!({
            "type": "usage",
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
            "cache_creation_input_tokens": usage.cache_creation_input_tokens,
            "cache_read_input_tokens": usage.cache_read_input_tokens,
        }),
        AssistantEvent::PromptCache(event) => json!({
            "type": "prompt_cache",
            "unexpected": event.unexpected,
            "reason": event.reason,
            "previous_cache_read_input_tokens": event.previous_cache_read_input_tokens,
            "current_cache_read_input_tokens": event.current_cache_read_input_tokens,
            "token_drop": event.token_drop,
        }),
        AssistantEvent::MessageStop => json!({ "type": "message_stop" }),
    };
    let _ = emit_stream_json_line(stdout, payload);
}

fn emit_stream_json_line(stdout: &mut impl Write, payload: Value) -> Result<(), String> {
    writeln!(
        stdout,
        "{}",
        serde_json::to_string(&payload).map_err(|error| error.to_string())?
    )
    .map_err(|error| error.to_string())?;
    stdout.flush().map_err(|error| error.to_string())
}

fn emit_bridge_status(emitter: &BridgeEmitter, status: &str) {
    let _ = emitter.emit(json!({
        "type": "turn_status",
        "text": status,
    }));
}

impl BridgeEmitter {
    fn new() -> Self {
        Self {
            stdout: Arc::new(Mutex::new(io::BufWriter::new(io::stdout()))),
        }
    }

    fn emit(&self, payload: Value) -> Result<(), String> {
        let mut stdout = self
            .stdout
            .lock()
            .map_err(|_| "bridge stdout lock poisoned".to_string())?;
        emit_stream_json_line(&mut *stdout, payload)
    }
}

fn emit_bridge_assistant_event(emitter: &BridgeEmitter, event: &AssistantEvent) {
    let payload = match event {
        AssistantEvent::TextDelta(text) => json!({ "type": "content_delta", "delta": text }),
        AssistantEvent::ToolUse { id, name, input } => json!({
            "type": "tool_start",
            "id": id,
            "name": name,
            "input": parse_tool_input(input),
        }),
        AssistantEvent::Usage(usage) => json!({
            "type": "usage",
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
            "cache_creation_input_tokens": usage.cache_creation_input_tokens,
            "cache_read_input_tokens": usage.cache_read_input_tokens,
        }),
        AssistantEvent::PromptCache(event) => json!({
            "type": "prompt_cache",
            "unexpected": event.unexpected,
            "reason": event.reason,
            "previous_cache_read_input_tokens": event.previous_cache_read_input_tokens,
            "current_cache_read_input_tokens": event.current_cache_read_input_tokens,
            "token_drop": event.token_drop,
        }),
        AssistantEvent::MessageStop => json!({ "type": "message_stop" }),
    };
    let _ = emitter.emit(payload);
}

fn parse_bridge_input(line: &str) -> Option<BridgeInput> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let value = match serde_json::from_str::<Value>(trimmed) {
        Ok(value) => value,
        Err(error) => {
            return Some(BridgeInput::ParseError(format!(
                "invalid ui bridge JSON: {error}"
            )));
        }
    };
    let message_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    Some(match message_type.as_str() {
        "ui_ready" => BridgeInput::UiReady,
        "user_turn" => BridgeInput::UserTurn(
            value
                .get("prompt")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ),
        "slash_command" => BridgeInput::SlashCommand(
            value
                .get("input")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ),
        "permission_response" => BridgeInput::PermissionResponse {
            decision: value
                .get("decision")
                .and_then(Value::as_str)
                .unwrap_or("deny")
                .to_string(),
            updated_input: value
                .get("updated_input")
                .and_then(Value::as_str)
                .map(str::to_string),
            reason: value
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        "shutdown" => BridgeInput::Shutdown,
        other => BridgeInput::ParseError(format!("unknown ui bridge command `{other}`")),
    })
}

fn bridge_session_payload(
    event_type: &str,
    surface: &RuntimeSurface,
    tool_surface: &ToolSurface,
    session: &Session,
    permission_mode: PermissionMode,
) -> Value {
    let usage = UsageTracker::from_session(session).cumulative_usage();
    let context_window_tokens = runtime::DEFAULT_CONTEXT_WINDOW_TOKENS;
    let context_breakdown = estimate_injected_context_breakdown(surface, tool_surface, session);
    let context_tokens =
        context_breakdown.session + context_breakdown.system + context_breakdown.tools;
    let context_percent = if context_window_tokens == 0 {
        0.0
    } else {
        context_tokens as f64 / context_window_tokens as f64 * 100.0
    };
    json!({
        "type": event_type,
        "session": {
            "id": session.session_id,
            "path": session.persistence_path().map(|path| path.display().to_string()),
            "message_count": session.messages.len(),
            "compaction_count": session.compaction.as_ref().map_or(0, |compaction| compaction.count),
        },
        "model": surface.model,
        "profile": surface.profile_name,
        "provider": surface.provider_label,
        "wire_model": surface.wire_model,
        "permission_mode": permission_mode.as_str(),
        "workspace": surface.cwd.display().to_string(),
        "usage": {
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
            "cache_creation_input_tokens": usage.cache_creation_input_tokens,
            "cache_read_input_tokens": usage.cache_read_input_tokens,
        },
        "context_window_tokens": context_window_tokens,
        "context_tokens": context_tokens,
        "context_percent": context_percent,
        "context_breakdown": {
            "session_tokens": context_breakdown.session,
            "system_tokens": context_breakdown.system,
            "tool_tokens": context_breakdown.tools,
        },
        "auto_compact_percent": runtime::AUTO_COMPACTION_CONTEXT_PERCENT,
    })
}

struct InjectedContextBreakdown {
    session: usize,
    system: usize,
    tools: usize,
}

fn estimate_injected_context_breakdown(
    surface: &RuntimeSurface,
    tool_surface: &ToolSurface,
    session: &Session,
) -> InjectedContextBreakdown {
    InjectedContextBreakdown {
        session: runtime::estimate_session_tokens(session),
        system: estimate_text_tokens(&surface.system_prompt.join("\n\n")),
        tools: estimate_tool_definitions_tokens(&tool_surface.display_definitions),
    }
}

fn estimate_text_tokens(text: &str) -> usize {
    text.len() / 4 + usize::from(!text.is_empty())
}

fn estimate_tool_definitions_tokens(definitions: &[ToolDefinition]) -> usize {
    definitions
        .iter()
        .map(|definition| {
            estimate_text_tokens(&definition.name)
                + estimate_text_tokens(definition.description.as_deref().unwrap_or_default())
                + estimate_text_tokens(&definition.input_schema.to_string())
        })
        .sum()
}

fn spawn_bridge_stdin_reader(sender: Sender<BridgeInput>) {
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    if let Some(message) = parse_bridge_input(&line) {
                        if sender.send(message).is_err() {
                            break;
                        }
                    }
                }
                Err(error) => {
                    let _ = sender.send(BridgeInput::ParseError(format!(
                        "failed to read ui bridge input: {error}"
                    )));
                    break;
                }
            }
        }
        let _ = sender.send(BridgeInput::Shutdown);
    });
}

fn parse_tool_input(input: &str) -> Value {
    serde_json::from_str(input).unwrap_or_else(|_| json!({ "raw": input }))
}

fn resolve_prompt_input(prompt: Option<String>) -> Result<String, String> {
    if let Some(prompt) = prompt {
        return Ok(prompt);
    }
    if io::stdin().is_terminal() {
        return Ok(String::new());
    }
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    Ok(buffer.trim().to_string())
}

fn render_status_report(
    surface: &RuntimeSurface,
    permission_mode: PermissionMode,
    session_path: Option<&Path>,
) -> String {
    let mut lines = vec![
        "Status".to_string(),
        format!("  Model            {}", surface.model),
        format!("  Wire model       {}", surface.wire_model),
        format!("  Permission mode  {}", permission_mode.as_str()),
        format!("  Profile          {}", surface.profile_name),
        format!("  Provider         {}", surface.provider_label),
        format!("  Provider API     {:?}", surface.provider_api),
        format!("  Base URL         {}", surface.base_url),
        format!("  Workspace        {}", surface.cwd.display()),
    ];
    if let Some(path) = session_path {
        lines.push(format!("  Session          {}", path.display()));
    }
    lines.join("\n")
}

fn unsupported_slash_command_message(input: &str, surface: &str) -> String {
    let command = input.split_whitespace().next().unwrap_or("/unknown").trim();
    format!(
        "{surface} does not expose {command} on the current Rust surface. Use /help for supported commands."
    )
}

fn render_config_report(config: &RuntimeConfig, section: Option<&str>) -> String {
    let rendered_config = redact_sensitive_json_string(&config.as_json().render());
    let mut lines = vec!["Config".to_string()];
    for entry in config.loaded_entries() {
        lines.push(format!(
            "  Loaded {:?}    {}",
            entry.source,
            entry.path.display()
        ));
    }
    match section {
        Some("model") => {
            lines.push(String::new());
            lines.push(
                config
                    .get_path(&["model"])
                    .map_or_else(|| "No model section.".to_string(), |value| value.render()),
            );
        }
        Some("hooks") => {
            lines.push(String::new());
            lines.push(format!(
                "{{\"pre_tool_use\":{},\"post_tool_use\":{},\"post_tool_use_failure\":{}}}",
                json!(config.hooks().pre_tool_use()).to_string(),
                json!(config.hooks().post_tool_use()).to_string(),
                json!(config.hooks().post_tool_use_failure()).to_string(),
            ));
        }
        Some("plugins") => {
            lines.push(String::new());
            lines.push(rendered_config.clone());
        }
        Some("env") | None | Some(_) => {
            lines.push(String::new());
            lines.push(rendered_config);
        }
    }
    lines.join("\n")
}

fn redact_sensitive_json_string(input: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<Value>(input) else {
        return input.to_string();
    };
    redact_sensitive_json_value(&mut value);
    serde_json::to_string(&value).unwrap_or_else(|_| input.to_string())
}

fn redact_sensitive_json_value(value: &mut Value) {
    match value {
        Value::Object(entries) => {
            for (key, child) in entries.iter_mut() {
                if is_sensitive_config_key(key) {
                    *child = Value::String("<redacted>".to_string());
                } else {
                    redact_sensitive_json_value(child);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_sensitive_json_value(item);
            }
        }
        _ => {}
    }
}

fn is_sensitive_config_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    matches!(
        normalized.as_str(),
        "apikey"
            | "accesstoken"
            | "refreshtoken"
            | "clientsecret"
            | "password"
            | "authorization"
            | "bearertoken"
    )
}

fn render_profile_report(
    config: &RuntimeConfig,
    action: Option<&str>,
    target: Option<&str>,
) -> Result<String, String> {
    match action {
        None | Some("list") => {
            let mut lines = vec!["Profiles".to_string()];
            for name in ProfileResolver::available_profile_names(config) {
                lines.push(format!("  - {name}"));
            }
            Ok(lines.join("\n"))
        }
        Some("show") => {
            let resolved = if let Some(target) = target {
                ProfileResolver::resolve_named(config, target, None)
            } else {
                ProfileResolver::resolve(config, None, None)
            }
            .map_err(|error| error.to_string())?;
            Ok(format!(
                "Profile\n  Name             {}\n  Model            {}\n  Base URL         {}\n  Credential       {}\n  Supports tools   {}\n  Supports stream  {}",
                resolved.profile_name,
                resolved.model,
                resolved.base_url.clone().unwrap_or_else(|| "<missing>".to_string()),
                resolved.credential.env_name,
                resolved.profile.supports_tools,
                resolved.profile.supports_streaming,
            ))
        }
        Some(other) => Err(format!("unsupported profile action: {other}")),
    }
}

fn render_sandbox_report(surface: &RuntimeSurface) -> String {
    let status = resolve_sandbox_status(surface.runtime_config.sandbox(), &surface.cwd);
    format!(
        "Sandbox\n  Enabled          {}\n  Active           {}\n  Namespace        {}\n  Network          {}\n  Filesystem       {}\n  Allowed mounts   {}\n  Fallback         {}",
        status.enabled,
        status.active,
        status.namespace_active,
        status.network_active,
        status.filesystem_mode.as_str(),
        if status.allowed_mounts.is_empty() {
            "-".to_string()
        } else {
            status.allowed_mounts.join(", ")
        },
        status
            .fallback_reason
            .unwrap_or_else(|| "none".to_string()),
    )
}

fn render_doctor_report(
    surface: &RuntimeSurface,
    permission_mode: PermissionMode,
) -> Result<String, String> {
    let status = render_status_report(surface, permission_mode, None);
    let sandbox = render_sandbox_report(surface);
    let commands_snapshot =
        build_command_registry_snapshot(&CommandRegistryContext::cli_local(), &[]);
    let profile = render_profile_report(&surface.runtime_config, Some("show"), None)?;
    Ok(format!(
        "Doctor\n\n{}\n\n{}\n\n{}\n\nCommands\n  Session surface  {}\n  Filtered out     {}",
        status,
        profile,
        sandbox,
        commands_snapshot.session_commands.len(),
        commands_snapshot.filtered_out_commands.len(),
    ))
}

fn render_memory_report() -> Result<String, String> {
    let memories = load_user_memories().map_err(|error| error.to_string())?;
    if memories.is_empty() {
        return Ok("Memory\n  No memories loaded".to_string());
    }
    let mut lines = vec!["Memory".to_string()];
    for memory in memories {
        lines.push(format!(
            "  - {} ({}) {}",
            memory.name,
            memory.memory_type.as_str(),
            memory.description
        ));
    }
    Ok(lines.join("\n"))
}

fn build_plugin_manager(config: &RuntimeConfig, config_home: &Path) -> PluginManager {
    let plugin_config = config.plugins();
    let mut manager_config = PluginManagerConfig::new(config_home.to_path_buf());
    manager_config.enabled_plugins = plugin_config.enabled_plugins().clone();
    manager_config.external_dirs = plugin_config
        .external_directories()
        .iter()
        .map(PathBuf::from)
        .collect();
    manager_config.install_root = plugin_config.install_root().map(PathBuf::from);
    manager_config.registry_path = plugin_config.registry_path().map(PathBuf::from);
    manager_config.bundled_root = plugin_config.bundled_root().map(PathBuf::from);
    PluginManager::new(manager_config)
}

fn current_date_string() -> String {
    Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn join_optional_args(args: &[&str]) -> Option<String> {
    (!args.is_empty()).then(|| args.join(" "))
}

fn read_next_line(
    lines: &mut impl Iterator<Item = io::Result<String>>,
) -> Result<Option<String>, String> {
    match lines.next() {
        Some(Ok(line)) => Ok(Some(line)),
        Some(Err(error)) => Err(error.to_string()),
        None => Ok(None),
    }
}

fn final_assistant_text(summary: &runtime::TurnSummary) -> String {
    let text = summary
        .assistant_messages
        .last()
        .map(|message| {
            message
                .blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    if text.trim().is_empty() {
        NO_CONTENT_MESSAGE.to_string()
    } else {
        text
    }
}

impl<'a> FrontlineApiClient<'a> {
    fn new(
        surface: &RuntimeSurface,
        tool_surface: &ToolSurface,
        effort: Option<api::ReasoningEffort>,
        event_observer: Option<&'a mut dyn FnMut(&AssistantEvent)>,
        status_observer: Option<&'a mut dyn FnMut(&str)>,
    ) -> Result<Self, String> {
        let client =
            OpenAiCompatClient::new(surface.api_key.clone(), OpenAiCompatConfig::saicode())
                .with_base_url(surface.base_url.clone())
                .with_request_timeout(Duration::from_millis(surface.request_timeout_ms))
                .with_retry_policy(
                    surface.max_retries,
                    Duration::from_millis(200),
                    Duration::from_secs(2),
                );
        Ok(Self {
            runtime: tokio::runtime::Runtime::new().map_err(|error| error.to_string())?,
            client,
            model: surface.wire_model.clone(),
            effort,
            tool_definitions: tool_surface.display_definitions.clone(),
            event_observer,
            status_observer,
        })
    }
}

impl ApiClient for FrontlineApiClient<'_> {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        let explicit_tool_matches =
            explicit_tool_matches_for_request(&request.messages, &self.tool_definitions);
        let has_tool_results = request_contains_tool_results(&request.messages);
        let scoped_tool_definitions = if !explicit_tool_matches.is_empty() {
            explicit_tool_matches.clone()
        } else {
            self.tool_definitions.clone()
        };
        let mut system_prompt = request.system_prompt.clone();
        if !explicit_tool_matches.is_empty() && !has_tool_results {
            let tool_names = explicit_tool_matches
                .iter()
                .map(|definition| definition.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            system_prompt.push(format!(
                "The user explicitly requested these available tools: {tool_names}. Restrict yourself to those tools for this request and call one or more of them before answering. Do not claim that the interface cannot use them because they are available in this session."
            ));
            system_prompt.push(
                "When the user provided concrete values for required tool inputs, pass those values through exactly in the tool call instead of leaving arguments empty.".to_string(),
            );
        }
        let tool_surface_enabled = !scoped_tool_definitions.is_empty();
        let allow_tool_schema_fallback = explicit_tool_matches.is_empty() && tool_surface_enabled;
        let effective_model = preferred_wire_model_for_request(&self.model, tool_surface_enabled);
        let tool_choice = if !has_tool_results && explicit_tool_matches.len() == 1 {
            Some(ToolChoice::Tool {
                name: explicit_tool_matches[0].name.clone(),
            })
        } else if !self.tool_definitions.is_empty() {
            Some(ToolChoice::Auto)
        } else {
            None
        };
        let message_request = MessageRequest {
            model: effective_model.clone(),
            max_tokens: None,
            messages: convert_messages(&request.messages),
            system: (!system_prompt.is_empty()).then(|| system_prompt.join("\n\n")),
            tools: (!scoped_tool_definitions.is_empty()).then_some(scoped_tool_definitions),
            tool_choice,
            reasoning_effort: self.effort,
            stream: true,
        };

        self.runtime.block_on(async {
            if let Some(observer) = self.status_observer.as_deref_mut() {
                if effective_model != self.model {
                    observer("Tool-capable runtime is using gpt-5.4-mini for stable execution…");
                }
                observer("Sending request to model…");
            }
            let mut retry_request = message_request.clone();
            let mut attempted_fast_model_fallback = false;
            let mut attempted_tool_fallback = false;
            let mut stream = loop {
                match self.client.stream_message(&retry_request).await {
                    Ok(stream) => break stream,
                    Err(error)
                        if !attempted_fast_model_fallback
                            && should_retry_with_fast_model(&error, &retry_request.model) =>
                    {
                        attempted_fast_model_fallback = true;
                        retry_request.model = DEFAULT_FAST_FALLBACK_MODEL_ID
                            .trim_start_matches("cpa/")
                            .to_string();
                        if let Some(observer) = self.status_observer.as_deref_mut() {
                            observer("Default qwen model unavailable; retrying with gpt-5.4-mini…");
                        }
                    }
                    Err(error)
                        if !attempted_tool_fallback
                            && should_retry_without_tools(&error, allow_tool_schema_fallback) =>
                    {
                        attempted_tool_fallback = true;
                        retry_request.tools = None;
                        retry_request.tool_choice = None;
                        if let Some(observer) = self.status_observer.as_deref_mut() {
                            observer("Model rejected tool schema; retrying without tools…");
                        }
                    }
                    Err(error) => return Err(RuntimeError::new(error.to_string())),
                }
            };
            if let Some(observer) = self.status_observer.as_deref_mut() {
                observer("Waiting for model output…");
            }
            let mut events = Vec::new();
            let mut pending_tools = BTreeMap::new();
            let mut saw_stop = false;
            let mut saw_activity = false;

            while let Some(event) = stream
                .next_event()
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?
            {
                match event {
                    ApiStreamEvent::MessageStart(start) => {
                        for block in start.message.content {
                            let before = events.len();
                            push_output_block(block, 0, &mut events, &mut pending_tools, true);
                            if let Some(observer) = self.event_observer.as_deref_mut() {
                                for event in &events[before..] {
                                    observer(event);
                                }
                            }
                        }
                    }
                    ApiStreamEvent::ContentBlockStart(start) => {
                        let before = events.len();
                        push_output_block(
                            start.content_block,
                            start.index,
                            &mut events,
                            &mut pending_tools,
                            true,
                        );
                        if let Some(observer) = self.event_observer.as_deref_mut() {
                            for event in &events[before..] {
                                observer(event);
                            }
                        }
                    }
                    ApiStreamEvent::ContentBlockDelta(delta) => match delta.delta {
                        ContentBlockDelta::TextDelta { text } => {
                            if !text.is_empty() {
                                if !saw_activity {
                                    if let Some(observer) = self.status_observer.as_deref_mut() {
                                        observer("Receiving response…");
                                    }
                                    saw_activity = true;
                                }
                                let event = AssistantEvent::TextDelta(text);
                                if let Some(observer) = self.event_observer.as_deref_mut() {
                                    observer(&event);
                                }
                                events.push(event);
                            }
                        }
                        ContentBlockDelta::InputJsonDelta { partial_json } => {
                            if let Some((_, _, input)) = pending_tools.get_mut(&delta.index) {
                                input.push_str(&partial_json);
                            }
                        }
                        ContentBlockDelta::ThinkingDelta { .. }
                        | ContentBlockDelta::SignatureDelta { .. } => {}
                    },
                    ApiStreamEvent::ContentBlockStop(stop) => {
                        if let Some((id, name, input)) = pending_tools.remove(&stop.index) {
                            let input =
                                repair_tool_input_from_latest_user_text(&input, &request.messages);
                            if let Some(observer) = self.status_observer.as_deref_mut() {
                                observer(&format!("Running tool: {name}"));
                            }
                            saw_activity = true;
                            let event = AssistantEvent::ToolUse { id, name, input };
                            if let Some(observer) = self.event_observer.as_deref_mut() {
                                observer(&event);
                            }
                            events.push(event);
                        }
                    }
                    ApiStreamEvent::MessageDelta(delta) => {
                        let event = AssistantEvent::Usage(delta.usage.token_usage());
                        if let Some(observer) = self.event_observer.as_deref_mut() {
                            observer(&event);
                        }
                        events.push(event);
                    }
                    ApiStreamEvent::MessageStop(_) => {
                        saw_stop = true;
                        if let Some(observer) = self.status_observer.as_deref_mut() {
                            observer("Model response complete.");
                        }
                        let event = AssistantEvent::MessageStop;
                        if let Some(observer) = self.event_observer.as_deref_mut() {
                            observer(&event);
                        }
                        events.push(event);
                    }
                }
            }

            if !saw_stop {
                let event = AssistantEvent::MessageStop;
                if let Some(observer) = self.event_observer.as_deref_mut() {
                    observer(&event);
                }
                events.push(event);
            }

            Ok(events)
        })
    }
}

fn should_retry_without_tools(error: &ApiError, allow_tool_schema_fallback: bool) -> bool {
    if !allow_tool_schema_fallback {
        return false;
    }

    match error {
        ApiError::Api { status, body, .. } => {
            status.as_u16() == 400
                && body.contains("Function id")
                && body.contains("DEGRADED function cannot be invoked")
        }
        ApiError::RetriesExhausted { last_error, .. } => {
            should_retry_without_tools(last_error, allow_tool_schema_fallback)
        }
        _ => false,
    }
}

fn preferred_wire_model_for_request(wire_model: &str, tool_surface_enabled: bool) -> String {
    if tool_surface_enabled && wire_model.eq_ignore_ascii_case("qwen/qwen3.5-122b-a10b") {
        return "gpt-5.4-mini".to_string();
    }

    wire_model.to_string()
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

fn explicit_tool_matches_for_request(
    messages: &[ConversationMessage],
    tool_definitions: &[ToolDefinition],
) -> Vec<ToolDefinition> {
    let Some(user_text) = latest_user_text(messages) else {
        return Vec::new();
    };
    let normalized_prompt = normalize_tool_hint_text(&user_text);
    let explicit_tool_keys =
        explicit_tool_name_keys_from_prompt(&normalized_prompt, tool_definitions);
    tool_definitions
        .iter()
        .filter(|definition| {
            explicit_tool_keys.contains(&normalize_tool_hint_name(&definition.name))
        })
        .cloned()
        .collect::<Vec<_>>()
}

fn request_contains_tool_results(messages: &[ConversationMessage]) -> bool {
    messages.iter().any(|message| {
        message
            .blocks
            .iter()
            .any(|block| matches!(block, ContentBlock::ToolResult { .. }))
    })
}

fn latest_user_text(messages: &[ConversationMessage]) -> Option<String> {
    messages
        .iter()
        .rev()
        .find(|message| message.role == MessageRole::User)
        .map(|message| {
            message
                .blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|text| !text.trim().is_empty())
}

fn repair_tool_input_from_latest_user_text(
    input: &str,
    messages: &[ConversationMessage],
) -> String {
    let Some(user_text) = latest_user_text(messages) else {
        return input.to_string();
    };
    let Some(candidate) = extract_first_json_object(&user_text) else {
        return input.to_string();
    };
    let Some(candidate_object) = candidate.as_object().cloned() else {
        return input.to_string();
    };

    let mut repaired = match serde_json::from_str::<Value>(input).ok() {
        Some(Value::Object(object)) => Value::Object(object),
        Some(_) => candidate.clone(),
        None => candidate.clone(),
    };

    if let Some(repaired_object) = repaired.as_object_mut() {
        for (key, value) in &candidate_object {
            let should_replace = repaired_object
                .get(key)
                .is_none_or(|current| match current {
                    Value::Null => true,
                    Value::String(text) => {
                        let normalized = text.trim().to_ascii_lowercase();
                        normalized.is_empty()
                            || matches!(
                                normalized.as_str(),
                                "ignored" | "ignore" | "placeholder" | "dummy"
                            )
                    }
                    Value::Object(object) => object.is_empty(),
                    Value::Array(array) => array.is_empty(),
                    _ => false,
                });
            if should_replace {
                repaired_object.insert(key.clone(), value.clone());
            }
        }
    }

    match serde_json::to_string(&repaired) {
        Ok(serialized) => serialized,
        Err(_) => input.to_string(),
    }
}

fn extract_first_json_object(text: &str) -> Option<Value> {
    let bytes = text.as_bytes();
    let mut start = 0usize;
    while start < bytes.len() {
        let Some(open_offset) = text[start..].find('{') else {
            return None;
        };
        let open = start + open_offset;
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escape = false;
        for (offset, ch) in text[open..].char_indices() {
            if in_string {
                if escape {
                    escape = false;
                    continue;
                }
                match ch {
                    '\\' => escape = true,
                    '"' => in_string = false,
                    _ => {}
                }
                continue;
            }

            match ch {
                '"' => in_string = true,
                '{' => depth += 1,
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let end = open + offset + ch.len_utf8();
                        let slice = &text[open..end];
                        if let Ok(value) = serde_json::from_str::<Value>(slice) {
                            if value.is_object() {
                                return Some(value);
                            }
                        }
                        start = end;
                        break;
                    }
                }
                _ => {}
            }
        }
        if depth > 0 {
            return None;
        }
    }
    None
}

fn normalize_tool_hint_text(value: &str) -> String {
    let collapsed = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>();
    format!(
        " {} ",
        collapsed.split_whitespace().collect::<Vec<_>>().join(" ")
    )
}

fn normalize_tool_hint_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect::<String>()
}

fn explicit_tool_name_keys_from_prompt(
    normalized_prompt: &str,
    tool_definitions: &[ToolDefinition],
) -> BTreeSet<String> {
    const TRIGGERS: &[&str] = &["use", "using", "call", "run", "with", "via"];
    const CONNECTORS: &[&str] = &["and", "or", "plus", "then"];
    const SKIP_TOKENS: &[&str] = &[
        "the",
        "a",
        "an",
        "tool",
        "tools",
        "available",
        "builtin",
        "built",
        "in",
    ];

    let tool_keys = tool_definitions
        .iter()
        .map(|definition| normalize_tool_hint_name(&definition.name))
        .collect::<BTreeSet<_>>();
    let tokens = normalized_prompt.split_whitespace().collect::<Vec<_>>();
    let mut found = BTreeSet::new();

    if !tokens.is_empty() {
        collect_explicit_tool_sequence(&tokens, 0, &tool_keys, &mut found);
    }

    for (index, token) in tokens.iter().enumerate() {
        if TRIGGERS.contains(token) {
            collect_explicit_tool_sequence(&tokens, index + 1, &tool_keys, &mut found);
            continue;
        }
        if CONNECTORS.contains(token) && index + 1 < tokens.len() {
            collect_explicit_tool_sequence(&tokens, index + 1, &tool_keys, &mut found);
            continue;
        }
        if SKIP_TOKENS.contains(token) && index == 0 {
            collect_explicit_tool_sequence(&tokens, index + 1, &tool_keys, &mut found);
        }
    }

    found
}

fn collect_explicit_tool_sequence(
    tokens: &[&str],
    start_index: usize,
    tool_keys: &BTreeSet<String>,
    found: &mut BTreeSet<String>,
) {
    const CONNECTORS: &[&str] = &["and", "or", "plus", "then"];
    const SKIP_TOKENS: &[&str] = &[
        "the",
        "a",
        "an",
        "tool",
        "tools",
        "available",
        "builtin",
        "built",
        "in",
    ];
    const BREAK_TOKENS: &[&str] = &["to", "for", "if", "reply", "inspect", "search", "fetch"];

    let mut saw_tool = false;
    for token in tokens.iter().skip(start_index) {
        if SKIP_TOKENS.contains(token) || CONNECTORS.contains(token) {
            continue;
        }
        if tool_keys.contains(*token) {
            found.insert((*token).to_string());
            saw_tool = true;
            continue;
        }
        if saw_tool || BREAK_TOKENS.contains(token) {
            break;
        }
        break;
    }
}

impl ToolExecutor for FrontlineToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        match self.permission_enforcer.check(tool_name, input) {
            EnforcementResult::Allowed => {}
            EnforcementResult::Denied { reason, .. } => {
                if let Some(emitter) = &self.event_emitter {
                    emit_bridge_status(
                        emitter,
                        &format!("Permission denied for tool: {tool_name}"),
                    );
                    let _ = emitter.emit(json!({
                        "type": "tool_error",
                        "name": tool_name,
                        "error": reason,
                    }));
                }
                return Err(ToolError::new(reason));
            }
        }
        let canonical = self
            .display_to_canonical
            .get(tool_name)
            .cloned()
            .unwrap_or_else(|| tool_name.to_string());
        let parsed = serde_json::from_str::<Value>(input)
            .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
        if canonical.starts_with("mcp__") {
            if let Some(emitter) = &self.event_emitter {
                emit_bridge_status(emitter, &format!("Running tool: {tool_name}"));
            }
            let result = execute_dynamic_mcp_tool(&self.runtime_config, &canonical, parsed)
                .map_err(ToolError::new);
            if let Some(emitter) = &self.event_emitter {
                match &result {
                    Ok(output) => {
                        emit_bridge_status(emitter, &format!("Tool completed: {tool_name}"));
                        let _ = emitter.emit(json!({
                            "type": "tool_result",
                            "name": tool_name,
                            "qualified_name": canonical,
                            "output": output,
                        }));
                    }
                    Err(error) => {
                        emit_bridge_status(emitter, &format!("Tool failed: {tool_name}"));
                        let _ = emitter.emit(json!({
                            "type": "tool_error",
                            "name": tool_name,
                            "qualified_name": canonical,
                            "error": error.to_string(),
                        }));
                    }
                }
            }
            return result;
        }
        if let Some(emitter) = &self.event_emitter {
            emit_bridge_status(emitter, &format!("Running tool: {tool_name}"));
        }
        let result = self
            .registry
            .execute(&canonical, &parsed)
            .map_err(ToolError::new);
        if let Some(emitter) = &self.event_emitter {
            match &result {
                Ok(output) => {
                    emit_bridge_status(emitter, &format!("Tool completed: {tool_name}"));
                    let _ = emitter.emit(json!({
                        "type": "tool_result",
                        "name": tool_name,
                        "qualified_name": canonical,
                        "output": output,
                    }));
                }
                Err(error) => {
                    emit_bridge_status(emitter, &format!("Tool failed: {tool_name}"));
                    let _ = emitter.emit(json!({
                        "type": "tool_error",
                        "name": tool_name,
                        "qualified_name": canonical,
                        "error": error.to_string(),
                    }));
                }
            }
        }
        result
    }
}

fn execute_dynamic_mcp_tool(
    runtime_config: &RuntimeConfig,
    canonical: &str,
    input: Value,
) -> Result<String, String> {
    let mut manager = McpServerManager::from_runtime_config(runtime_config);
    let rt = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    let tools = rt
        .block_on(manager.discover_tools())
        .map_err(|error| error.to_string())?;
    let resolved = tools
        .into_iter()
        .find(|tool| tool.qualified_name == canonical)
        .ok_or_else(|| format!("unknown MCP tool `{canonical}`"))?;
    let result = rt
        .block_on(manager.call_tool(canonical, Some(input)))
        .map_err(|error| error.to_string())?;
    serde_json::to_string_pretty(&json!({
        "server": resolved.server_name,
        "tool": resolved.raw_name,
        "qualified_tool": canonical,
        "result": result,
    }))
    .map_err(|error| error.to_string())
}

impl PermissionPrompter for CliPermissionPrompter {
    fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision {
        println!(
            "Permission required: {}  current={}  required={}",
            request.tool_name,
            request.current_mode.as_str(),
            request.required_mode.as_str()
        );
        if let Some(reason) = &request.reason {
            println!("Reason: {reason}");
        }
        print!("Allow? [y/N] ");
        let _ = io::stdout().flush();
        let mut answer = String::new();
        match io::stdin().read_line(&mut answer) {
            Ok(_) if matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") => {
                PermissionPromptDecision::Allow {
                    updated_input: None,
                }
            }
            Ok(_) => PermissionPromptDecision::Deny {
                reason: "permission denied by user".to_string(),
            },
            Err(error) => PermissionPromptDecision::Deny {
                reason: format!("permission prompt failed: {error}"),
            },
        }
    }
}

impl PermissionPrompter for BridgePermissionPrompter<'_> {
    fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision {
        let _ = self.emitter.emit(json!({
            "type": "permission_request",
            "tool_name": request.tool_name,
            "input": parse_tool_input(&request.input),
            "current_mode": request.current_mode.as_str(),
            "required_mode": request.required_mode.as_str(),
            "reason": request.reason,
        }));
        while let Ok(message) = self.receiver.recv() {
            match message {
                BridgeInput::PermissionResponse {
                    decision,
                    updated_input,
                    reason,
                } => {
                    let normalized = decision.trim().to_ascii_lowercase();
                    if matches!(normalized.as_str(), "allow" | "approved" | "yes" | "y") {
                        let _ = self.emitter.emit(json!({
                            "type": "permission_resolved",
                            "decision": "allow",
                            "tool_name": request.tool_name,
                        }));
                        return PermissionPromptDecision::Allow { updated_input };
                    }
                    let reason = reason.unwrap_or_else(|| "permission denied by user".to_string());
                    let _ = self.emitter.emit(json!({
                        "type": "permission_resolved",
                        "decision": "deny",
                        "tool_name": request.tool_name,
                        "reason": reason,
                    }));
                    return PermissionPromptDecision::Deny { reason };
                }
                BridgeInput::Shutdown => {
                    return PermissionPromptDecision::Deny {
                        reason: "ui requested shutdown while waiting for permission".to_string(),
                    };
                }
                BridgeInput::ParseError(message) => {
                    let _ = self.emitter.emit(json!({
                        "type": "error",
                        "message": message,
                    }));
                }
                BridgeInput::UiReady | BridgeInput::UserTurn(_) | BridgeInput::SlashCommand(_) => {
                    let _ = self.emitter.emit(json!({
                        "type": "error",
                        "message": "ui sent a non-permission command while permission was pending",
                    }));
                }
            }
        }
        PermissionPromptDecision::Deny {
            reason: "ui bridge disconnected during permission prompt".to_string(),
        }
    }
}

fn convert_messages(messages: &[ConversationMessage]) -> Vec<InputMessage> {
    messages
        .iter()
        .filter_map(|message| {
            let role = match message.role {
                MessageRole::System | MessageRole::User | MessageRole::Tool => "user",
                MessageRole::Assistant => "assistant",
            };
            let content = message
                .blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } => InputContentBlock::Text { text: text.clone() },
                    ContentBlock::ToolUse { id, name, input } => InputContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: serde_json::from_str(input)
                            .unwrap_or_else(|_| json!({ "raw": input })),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id,
                        output,
                        is_error,
                        ..
                    } => InputContentBlock::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: vec![ToolResultContentBlock::Text {
                            text: output.clone(),
                        }],
                        is_error: *is_error,
                    },
                })
                .collect::<Vec<_>>();
            (!content.is_empty()).then(|| InputMessage {
                role: role.to_string(),
                content,
            })
        })
        .collect()
}

fn push_output_block(
    block: OutputContentBlock,
    block_index: u32,
    events: &mut Vec<AssistantEvent>,
    pending_tools: &mut BTreeMap<u32, (String, String, String)>,
    streaming_tool_input: bool,
) {
    match block {
        OutputContentBlock::Text { text } => {
            if !text.is_empty() {
                events.push(AssistantEvent::TextDelta(text));
            }
        }
        OutputContentBlock::ToolUse { id, name, input } => {
            let initial_input = if streaming_tool_input
                && input.is_object()
                && input.as_object().is_some_and(Map::is_empty)
            {
                String::new()
            } else {
                input.to_string()
            };
            pending_tools.insert(block_index, (id, name, initial_input));
        }
        OutputContentBlock::Thinking { .. } | OutputContentBlock::RedactedThinking { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bridge_session_payload, build_tool_surface, collect_tool_option_values,
        execute_dynamic_mcp_tool, explicit_tool_matches_for_request, join_optional_args,
        looks_like_tool_option_value, parse_effort, parse_output_format, parse_permission_mode,
        preferred_wire_model_for_request, redact_sensitive_json_string, registry_display_name,
        repair_tool_input_from_latest_user_text, resolved_permission_mode,
        should_retry_with_fast_model, should_retry_without_tools, CliArgs, OutputFormat,
        RuntimeSurface, ToolSurface,
    };
    use api::{ApiError, ToolDefinition};
    use runtime::{
        ConfigLoader, ContentBlock, ConversationMessage, MessageRole, PermissionMode,
        RuntimeConfig, Session,
    };
    use saicode_frontline::recovery::WireApi;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tools::GlobalToolRegistry;

    #[test]
    fn display_tool_names_preserve_saicode_surface_for_hot_tools() {
        assert_eq!(registry_display_name("read_file"), "Read");
        assert_eq!(registry_display_name("write_file"), "Write");
        assert_eq!(registry_display_name("bash"), "Bash");
        assert_eq!(registry_display_name("WebSearch"), "WebSearch");
    }

    #[test]
    fn parses_modes_and_effort() {
        assert_eq!(
            parse_permission_mode("workspace-write").expect("mode"),
            runtime::PermissionMode::WorkspaceWrite
        );
        assert_eq!(
            parse_effort("xhigh").expect("effort"),
            api::ReasoningEffort::Max
        );
        assert_eq!(
            parse_output_format("stream-json").expect("stream-json"),
            OutputFormat::StreamJson
        );
    }

    #[test]
    fn joins_optional_args_losslessly() {
        assert_eq!(
            join_optional_args(&["show", "remote"]),
            Some("show remote".to_string())
        );
        assert_eq!(join_optional_args(&[]), None);
    }

    #[test]
    fn tool_option_parser_stops_before_prompt_words() {
        let args = vec![
            "-p".to_string(),
            "--allowedTools".to_string(),
            "Read".to_string(),
            "Use".to_string(),
            "Read".to_string(),
        ];
        let (values, next_index) = collect_tool_option_values(&args, 1);
        assert_eq!(values, vec!["Read".to_string()]);
        assert_eq!(next_index, 2);
    }

    #[test]
    fn tool_option_value_detection_accepts_known_names_and_rules() {
        assert!(looks_like_tool_option_value("Read"));
        assert!(looks_like_tool_option_value("Bash(git:status)"));
        assert!(looks_like_tool_option_value("Read,Grep"));
        assert!(!looks_like_tool_option_value("Use"));
    }

    #[test]
    fn defaults_to_danger_full_access() {
        let args = CliArgs {
            print: true,
            prompt: Some("hello".to_string()),
            model: None,
            effort: None,
            permission_mode: None,
            allowed_tools: Vec::new(),
            disallowed_tools: Vec::new(),
            system_prompt: None,
            append_system_prompt: None,
            output_format: OutputFormat::Text,
            resume: None,
            no_session_persistence: false,
            bare: false,
            process_command: None,
        };
        assert_eq!(
            resolved_permission_mode(&args, &RuntimeConfig::empty()),
            runtime::PermissionMode::DangerFullAccess
        );
    }

    #[test]
    fn narrows_to_single_explicitly_requested_tool_without_forcing_tool_choice() {
        let messages = vec![ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text {
                text: "Use Read to inspect rust/Cargo.toml and reply with only the package name."
                    .to_string(),
            }],
            usage: None,
        }];
        let definitions = vec![
            ToolDefinition {
                name: "Read".to_string(),
                description: None,
                input_schema: json!({}),
            },
            ToolDefinition {
                name: "Bash".to_string(),
                description: None,
                input_schema: json!({}),
            },
        ];
        let scoped = explicit_tool_matches_for_request(&messages, &definitions);
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].name, "Read");
    }

    #[test]
    fn captures_multiple_explicitly_requested_tools_in_one_phrase() {
        let messages = vec![ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text {
                text: "Use Read and Bash if needed to inspect config and run curl, then reply with compact JSON.".to_string(),
            }],
            usage: None,
        }];
        let definitions = vec![
            ToolDefinition {
                name: "Read".to_string(),
                description: None,
                input_schema: json!({}),
            },
            ToolDefinition {
                name: "Bash".to_string(),
                description: None,
                input_schema: json!({}),
            },
            ToolDefinition {
                name: "WebFetch".to_string(),
                description: None,
                input_schema: json!({}),
            },
        ];
        let scoped = explicit_tool_matches_for_request(&messages, &definitions);
        assert_eq!(scoped.len(), 2);
        assert!(scoped.iter().any(|definition| definition.name == "Read"));
        assert!(scoped.iter().any(|definition| definition.name == "Bash"));
    }

    #[test]
    fn keeps_full_tool_pool_when_prompt_does_not_name_one_tool() {
        let messages = vec![ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text {
                text: "Inspect rust/Cargo.toml and tell me the workspace package name.".to_string(),
            }],
            usage: None,
        }];
        let definitions = vec![
            ToolDefinition {
                name: "Read".to_string(),
                description: None,
                input_schema: json!({}),
            },
            ToolDefinition {
                name: "Bash".to_string(),
                description: None,
                input_schema: json!({}),
            },
        ];
        let scoped = explicit_tool_matches_for_request(&messages, &definitions);
        assert!(scoped.is_empty());
    }

    #[test]
    fn build_tool_surface_injects_dynamic_mcp_tools() {
        let root = std::env::temp_dir().join(format!(
            "saicode-cli-mcp-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let cwd = root.join("cwd");
        let home = root.join("home");
        fs::create_dir_all(&cwd).expect("cwd");
        fs::create_dir_all(home.join(".saicode")).expect("config home");
        let server_path = write_test_mcp_server(&root);
        fs::write(
            home.join(".saicode").join("config.json"),
            format!(
                r#"{{"mcpServers":{{"alpha":{{"command":"python3","args":["{}"]}}}}}}"#,
                server_path.display()
            ),
        )
        .expect("write config");

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &home);
        let runtime_config = ConfigLoader::default_for(&cwd)
            .load()
            .expect("runtime config");
        let args = CliArgs {
            print: true,
            prompt: Some("ping".to_string()),
            model: None,
            effort: None,
            permission_mode: None,
            allowed_tools: vec!["mcp__alpha__echo".to_string()],
            disallowed_tools: Vec::new(),
            system_prompt: None,
            append_system_prompt: None,
            output_format: OutputFormat::Text,
            resume: None,
            no_session_persistence: true,
            bare: true,
            process_command: None,
        };

        let surface =
            build_tool_surface(&args, &runtime_config, &home.join(".saicode")).expect("surface");
        assert!(surface
            .display_definitions
            .iter()
            .any(|tool| tool.name == "mcp__alpha__echo"));
        assert!(surface.mcp_tools.contains_key("mcp__alpha__echo"));

        match original_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn execute_dynamic_mcp_tool_routes_to_server() {
        let root = std::env::temp_dir().join(format!(
            "saicode-cli-mcp-exec-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let cwd = root.join("cwd");
        let home = root.join("home");
        fs::create_dir_all(&cwd).expect("cwd");
        fs::create_dir_all(home.join(".saicode")).expect("config home");
        let server_path = write_test_mcp_server(&root);
        fs::write(
            home.join(".saicode").join("config.json"),
            format!(
                r#"{{"mcpServers":{{"alpha":{{"command":"python3","args":["{}"]}}}}}}"#,
                server_path.display()
            ),
        )
        .expect("write config");

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &home);
        let runtime_config = ConfigLoader::default_for(&cwd)
            .load()
            .expect("runtime config");
        let output =
            execute_dynamic_mcp_tool(&runtime_config, "mcp__alpha__echo", json!({"text":"hello"}))
                .expect("mcp tool result");
        assert!(output.contains("echo:hello"));

        match original_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        let _ = fs::remove_dir_all(root);
    }

    fn write_test_mcp_server(root: &std::path::Path) -> std::path::PathBuf {
        let path = root.join("mcp_echo.py");
        fs::write(
            &path,
            [
                "import json, sys",
                "",
                "def send(obj):",
                "    payload = json.dumps(obj).encode()",
                "    sys.stdout.write(f'Content-Length: {len(payload)}\\r\\n\\r\\n')",
                "    sys.stdout.flush()",
                "    sys.stdout.buffer.write(payload)",
                "    sys.stdout.buffer.flush()",
                "",
                "while True:",
                "    headers = {}",
                "    while True:",
                "        line = sys.stdin.buffer.readline()",
                "        if not line:",
                "            raise SystemExit(0)",
                "        if line in (b'\\r\\n', b'\\n'):",
                "            break",
                "        key, value = line.decode().split(':', 1)",
                "        headers[key.strip().lower()] = value.strip()",
                "    length = int(headers['content-length'])",
                "    body = sys.stdin.buffer.read(length)",
                "    message = json.loads(body.decode())",
                "    method = message.get('method')",
                "    ident = message.get('id')",
                "    if method == 'initialize':",
                "        send({'jsonrpc':'2.0','id':ident,'result':{'protocolVersion':'2024-11-05','capabilities':{},'serverInfo':{'name':'alpha','version':'1.0.0'}}})",
                "    elif method == 'notifications/initialized':",
                "        pass",
                "    elif method == 'tools/list':",
                "        send({'jsonrpc':'2.0','id':ident,'result':{'tools':[{'name':'echo','description':'Echo text','inputSchema':{'type':'object','properties':{'text':{'type':'string'}},'required':['text']}}]}})",
                "    elif method == 'tools/call':",
                "        text = ((message.get('params') or {}).get('arguments') or {}).get('text', '')",
                "        send({'jsonrpc':'2.0','id':ident,'result':{'content':[{'type':'text','text':f'echo:{text}'}],'structuredContent':{'echoed':text}}})",
                "    else:",
                "        send({'jsonrpc':'2.0','id':ident,'error':{'code':-32601,'message':'method not found'}})",
            ]
            .join("\n"),
        )
        .expect("write mcp server");
        path
    }

    #[test]
    fn bridge_session_payload_reports_context_and_current_surface_model() {
        let mut session = Session::new();
        session.messages = vec![
            ConversationMessage::user_text("hello ".repeat(200)),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "world ".repeat(120),
            }]),
        ];
        let surface = RuntimeSurface {
            cwd: PathBuf::from("/tmp"),
            config_home: PathBuf::from("/tmp/.saicode"),
            runtime_config: RuntimeConfig::empty(),
            system_prompt: vec!["system".to_string()],
            profile_name: "cliproxyapi".to_string(),
            provider_label: "cpa".to_string(),
            base_url: "http://127.0.0.1:8317/v1".to_string(),
            api_key: "test".to_string(),
            request_timeout_ms: 30_000,
            max_retries: 1,
            model: "cpa/qwen/qwen3.5-122b-a10b".to_string(),
            wire_model: "qwen/qwen3.5-122b-a10b".to_string(),
            provider_api: WireApi::OpenAIChatCompletions,
        };
        let tool_surface = ToolSurface {
            registry: GlobalToolRegistry::builtin(),
            display_definitions: vec![ToolDefinition {
                name: "Read".to_string(),
                description: Some("Read a file".to_string()),
                input_schema: json!({"type":"object","properties":{"path":{"type":"string"}}}),
            }],
            display_to_canonical: BTreeMap::new(),
            permission_requirements: BTreeMap::new(),
            mcp_tools: BTreeMap::new(),
        };

        let payload = bridge_session_payload(
            "session_updated",
            &surface,
            &tool_surface,
            &session,
            PermissionMode::DangerFullAccess,
        );

        assert_eq!(payload["model"], "cpa/qwen/qwen3.5-122b-a10b");
        assert_eq!(payload["wire_model"], "qwen/qwen3.5-122b-a10b");
        assert!(payload["context_tokens"].as_u64().unwrap_or_default() > 0);
        assert!(
            payload["context_tokens"].as_u64().unwrap_or_default()
                > runtime::estimate_session_tokens(&session) as u64
        );
        assert_eq!(payload["context_window_tokens"], 270000);
        assert_eq!(payload["auto_compact_percent"], 80);
    }

    #[test]
    fn status_report_uses_runtime_permission_mode() {
        let surface = RuntimeSurface {
            cwd: PathBuf::from("/tmp"),
            config_home: PathBuf::from("/tmp/.saicode"),
            runtime_config: RuntimeConfig::empty(),
            system_prompt: vec!["system".to_string()],
            profile_name: "cliproxyapi".to_string(),
            provider_label: "cpa".to_string(),
            base_url: "http://127.0.0.1:8317/v1".to_string(),
            api_key: "test".to_string(),
            request_timeout_ms: 30_000,
            max_retries: 1,
            model: "cpa/qwen/qwen3.5-122b-a10b".to_string(),
            wire_model: "qwen/qwen3.5-122b-a10b".to_string(),
            provider_api: WireApi::OpenAIChatCompletions,
        };

        let report = super::render_status_report(
            &surface,
            PermissionMode::WorkspaceWrite,
            Some(Path::new("/tmp/session.jsonl")),
        );

        assert!(report.contains("Permission mode  workspace-write"));
        assert!(report.contains("Session          /tmp/session.jsonl"));
    }

    #[test]
    fn retries_without_tools_for_degraded_function_schema_errors() {
        let error = ApiError::Api {
            status: "400".parse().expect("status code"),
            error_type: None,
            message: None,
            body: "{\"status\":400,\"title\":\"Bad Request\",\"detail\":\"Function id 'abc': DEGRADED function cannot be invoked\"}".to_string(),
            retryable: false,
        };

        assert!(should_retry_without_tools(&error, true));
        assert!(!should_retry_without_tools(&error, false));
    }

    #[test]
    fn does_not_retry_without_tools_for_other_api_errors() {
        let error = ApiError::Api {
            status: "400".parse().expect("status code"),
            error_type: None,
            message: None,
            body: "{\"status\":400,\"title\":\"Bad Request\",\"detail\":\"model not found\"}"
                .to_string(),
            retryable: false,
        };

        assert!(!should_retry_without_tools(&error, true));
    }

    #[test]
    fn retries_qwen_degraded_model_errors_with_fast_model() {
        let error = ApiError::Api {
            status: "400".parse().expect("status code"),
            error_type: None,
            message: None,
            body: "{\"status\":400,\"title\":\"Bad Request\",\"detail\":\"Function id 'abc': DEGRADED function cannot be invoked\"}".to_string(),
            retryable: false,
        };

        assert!(should_retry_with_fast_model(
            &error,
            "qwen/qwen3.5-122b-a10b"
        ));
        assert!(!should_retry_with_fast_model(&error, "gpt-5.4-mini"));
    }

    #[test]
    fn tool_capable_runtime_prefers_fast_wire_model() {
        assert_eq!(
            preferred_wire_model_for_request("qwen/qwen3.5-122b-a10b", true),
            "gpt-5.4-mini"
        );
        assert_eq!(
            preferred_wire_model_for_request("gpt-5.4-mini", true),
            "gpt-5.4-mini"
        );
        assert_eq!(
            preferred_wire_model_for_request("qwen/qwen3.5-122b-a10b", false),
            "qwen/qwen3.5-122b-a10b"
        );
    }

    #[test]
    fn repairs_empty_tool_input_from_latest_user_json() {
        let messages = vec![ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text {
                text: "Call mcp__alpha__echo with JSON arguments {\"text\":\"acceptance\"}."
                    .to_string(),
            }],
            usage: None,
        }];

        assert_eq!(
            repair_tool_input_from_latest_user_text("{}", &messages),
            "{\"text\":\"acceptance\"}"
        );
        assert_eq!(
            repair_tool_input_from_latest_user_text("{\"text\":\"\"}", &messages),
            "{\"text\":\"acceptance\"}"
        );
        assert_eq!(
            repair_tool_input_from_latest_user_text("{\"text\":\"already\"}", &messages),
            "{\"text\":\"already\"}"
        );
        assert_eq!(
            repair_tool_input_from_latest_user_text("{\"text\":\"ignored\"}", &messages),
            "{\"text\":\"acceptance\"}"
        );
    }

    #[test]
    fn redacts_sensitive_values_in_rendered_config_json() {
        let rendered = redact_sensitive_json_string(
            r#"{"providers":{"cpa":{"apiKey":"secret","baseUrl":"http://127.0.0.1:8317/v1","headers":{"Authorization":"Bearer abc"}}},"oauth":{"clientSecret":"hidden"},"password":"pw","model":"cpa/gpt-5.4"}"#,
        );

        assert!(rendered.contains(r#""apiKey":"<redacted>""#));
        assert!(rendered.contains(r#""Authorization":"<redacted>""#));
        assert!(rendered.contains(r#""clientSecret":"<redacted>""#));
        assert!(rendered.contains(r#""password":"<redacted>""#));
        assert!(rendered.contains(r#""model":"cpa/gpt-5.4""#));
        assert!(!rendered.contains("secret"));
        assert!(!rendered.contains("Bearer abc"));
        assert!(!rendered.contains("hidden"));
        assert!(!rendered.contains(r#""password":"pw""#));
    }
}
