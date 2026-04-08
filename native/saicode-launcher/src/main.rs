use saicode_frontline::{local_tools, recovery};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

const NATIVE_LOCAL_TOOL_NAMES: &[&str] =
    &[
        "Read",
        "Grep",
        "Glob",
        "Bash",
        "Write",
        "Edit",
        "WebSearch",
        "WebFetch",
    ];
const LIGHTWEIGHT_HEADLESS_TOOL_NAMES: &[&str] = &[
    "Bash",
    "Glob",
    "Grep",
    "Read",
    "Edit",
    "Write",
    "WebFetch",
    "WebSearch",
];
const EXPLICIT_TOOL_HINT_NAMES: &[&str] = &[
    "Read",
    "Bash",
    "Write",
    "Edit",
    "Glob",
    "Grep",
    "Skill",
    "LSP",
    "WebFetch",
    "WebSearch",
    "TaskCreate",
    "TaskList",
    "TaskGet",
    "TaskStop",
    "TaskUpdate",
    "TaskOutput",
    "MCP",
    "ListMcpResources",
    "ReadMcpResource",
    "McpAuth",
];

const FAST_HELP_TEXT: &str = r#"Usage: saicode [options] [command] [prompt]

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Route {
    Help,
    Version,
    Recovery,
    NativeLocalTools,
    LightweightHeadless,
    FullCli,
}

impl Route {
    fn label(self) -> &'static str {
        match self {
            Route::Help => "help",
            Route::Version => "version",
            Route::Recovery => "recovery",
            Route::NativeLocalTools => "native-local-tools",
            Route::LightweightHeadless => "lightweight-headless",
            Route::FullCli => "full-cli",
        }
    }

}

fn is_env_truthy(value: Option<&str>) -> bool {
    matches!(
        value.map(|item| item.trim().to_ascii_lowercase()),
        Some(ref item) if matches!(item.as_str(), "1" | "true" | "yes" | "on")
    )
}

fn is_env_defined_falsy(value: Option<&str>) -> bool {
    matches!(
        value.map(|item| item.trim().to_ascii_lowercase()),
        Some(ref item) if matches!(item.as_str(), "0" | "false" | "no" | "off")
    )
}

fn normalize_tool_restriction_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| {
            value
                .split(|ch: char| ch == ',' || ch.is_whitespace())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| item.split('(').next().unwrap_or(item).to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn raw_tool_restriction_values(values: &[String]) -> Vec<String> {
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
        .collect()
}

fn contains_bash_rule_suffix(values: &[String]) -> bool {
    raw_tool_restriction_values(values)
        .iter()
        .any(|value| value.starts_with("Bash("))
}

fn uses_only_tool_names_from_set(values: &[String], allowed: &[&str]) -> bool {
    let normalized = normalize_tool_restriction_values(values);
    !normalized.is_empty()
        && normalized
            .iter()
            .all(|value| allowed.iter().any(|candidate| candidate == value))
}

fn uses_only_lightweight_headless_tools(values: &[String]) -> bool {
    uses_only_tool_names_from_set(values, LIGHTWEIGHT_HEADLESS_TOOL_NAMES)
}

fn uses_only_native_local_tools(values: &[String]) -> bool {
    uses_only_tool_names_from_set(values, NATIVE_LOCAL_TOOL_NAMES)
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

fn should_use_recovery_entrypoint(cli_args: &[String]) -> bool {
    if env::var("SAICODE_FORCE_RECOVERY_CLI").ok().as_deref() == Some("1") {
        return true;
    }

    let is_print_mode = cli_args.iter().any(|arg| arg == "-p" || arg == "--print");
    if !is_print_mode {
        return false;
    }

    let mut index = 0;
    while index < cli_args.len() {
        let arg = cli_args[index].as_str();
        match arg {
            "--" => break,
            "-p" | "--print" | "--bare" | "--dangerously-skip-permissions" | "-h" | "--help"
            | "-v" | "-V" | "--version" => {}
            "--model" | "--system-prompt" | "--system-prompt-file" | "--append-system-prompt" => {
                index += 1;
            }
            "--output-format" => {
                if cli_args.get(index + 1).map(String::as_str) == Some("stream-json") {
                    return false;
                }
                index += 1;
            }
            "--input-format"
            | "--include-hook-events"
            | "--include-partial-messages"
            | "--replay-user-messages"
            | "--tools"
            | "--allowedTools"
            | "--allowed-tools"
            | "--disallowedTools"
            | "--disallowed-tools"
            | "--permission-prompt-tool"
            | "--mcp-config"
            | "--sdk-url"
            | "--session-id"
            | "--continue"
            | "--resume"
            | "--fork-session"
            | "--max-turns"
            | "--max-budget-usd"
            | "--agent"
            | "--agents" => return false,
            _ if arg.starts_with('-') => return false,
            _ => {}
        }

        index += 1;
    }

    if prompt_explicitly_requests_tool(&extract_print_prompt_text(cli_args)) {
        return false;
    }

    true
}

fn extract_print_prompt_text(cli_args: &[String]) -> String {
    let mut parts = Vec::new();
    let mut index = 0;
    while index < cli_args.len() {
        let arg = cli_args[index].as_str();
        match arg {
            "--" => {
                parts.extend(cli_args[index + 1..].iter().cloned());
                break;
            }
            "--model"
            | "--system-prompt"
            | "--system-prompt-file"
            | "--append-system-prompt"
            | "--append-system-prompt-file"
            | "--output-format"
            | "--permission-mode" => {
                index += 1;
            }
            value if value.starts_with('-') => {}
            value => parts.push(value.to_string()),
        }
        index += 1;
    }
    parts.join(" ")
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
    format!(" {} ", collapsed.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn prompt_explicitly_requests_tool(prompt: &str) -> bool {
    let normalized_prompt = normalize_tool_hint_text(prompt);
    EXPLICIT_TOOL_HINT_NAMES.iter().any(|tool| {
        let tool = tool.to_ascii_lowercase();
        [
            format!(" use {tool} "),
            format!(" using {tool} "),
            format!(" call {tool} "),
            format!(" run {tool} "),
            format!(" with {tool} "),
            format!(" via {tool} "),
        ]
        .iter()
        .any(|needle| normalized_prompt.contains(needle))
    })
}

fn is_restricted_tool_print_candidate(
    cli_args: &[String],
    accepts_tools: fn(&[String]) -> bool,
) -> bool {
    if is_env_defined_falsy(env::var("SAICODE_AUTO_BARE_PRINT").ok().as_deref()) {
        return false;
    }

    let is_print_mode = cli_args.iter().any(|arg| arg == "-p" || arg == "--print");
    if !is_print_mode {
        return false;
    }

    let mut saw_tool_restriction = false;
    let mut index = 0;

    while index < cli_args.len() {
        let arg = cli_args[index].as_str();
        match arg {
            "--" => break,
            "-p" | "--print" | "--bare" | "--dangerously-skip-permissions"
            | "--allow-dangerously-skip-permissions" => {}
            "-h" | "--help" | "-v" | "-V" | "--version" => return false,
            "--model"
            | "--system-prompt"
            | "--system-prompt-file"
            | "--append-system-prompt"
            | "--append-system-prompt-file"
            | "--permission-mode"
            | "--fallback-model"
            | "--json-schema"
            | "--max-turns"
            | "--max-budget-usd"
            | "--task-budget"
            | "--name"
            | "-n" => {
                index += 1;
            }
            "--output-format" => {
                if cli_args.get(index + 1).map(String::as_str) == Some("stream-json") {
                    return false;
                }
                index += 1;
            }
            "--tools" | "--allowedTools" | "--allowed-tools" => {
                let (values, next_index) = collect_variadic_option_values(cli_args, index);
                if !accepts_tools(&values) {
                    return false;
                }
                saw_tool_restriction = true;
                index = next_index;
            }
            "--disallowedTools"
            | "--disallowed-tools"
            | "--input-format"
            | "--include-hook-events"
            | "--include-partial-messages"
            | "--replay-user-messages"
            | "--permission-prompt-tool"
            | "--mcp-config"
            | "--sdk-url"
            | "--session-id"
            | "--continue"
            | "--resume"
            | "-c"
            | "-r"
            | "--fork-session"
            | "--agent"
            | "--agents"
            | "--plugin-dir"
            | "--add-dir"
            | "--settings"
            | "--strict-mcp-config"
            | "--ide"
            | "--init"
            | "--init-only"
            | "--maintenance" => return false,
            _ if arg.starts_with('-') => return false,
            _ => {}
        }

        index += 1;
    }

    saw_tool_restriction
}

fn should_use_lightweight_headless_print_entrypoint(cli_args: &[String]) -> bool {
    if cli_args.iter().any(|arg| arg == "--bare") {
        return false;
    }
    is_restricted_tool_print_candidate(cli_args, uses_only_lightweight_headless_tools)
}

fn should_use_native_local_tools_entrypoint(cli_args: &[String]) -> bool {
    let is_print_mode = cli_args.iter().any(|arg| arg == "-p" || arg == "--print");
    if !is_print_mode {
        return false;
    }

    let mut saw_tool_restriction = false;
    let mut index = 0;

    while index < cli_args.len() {
        let arg = cli_args[index].as_str();
        match arg {
            "--" => break,
            "-p" | "--print" | "--bare" | "--dangerously-skip-permissions"
            | "--allow-dangerously-skip-permissions" => {}
            "-h" | "--help" | "-v" | "-V" | "--version" => return false,
            "--model"
            | "--system-prompt"
            | "--system-prompt-file"
            | "--append-system-prompt"
            | "--append-system-prompt-file"
            | "--max-turns"
            | "--permission-mode" => {
                index += 1;
            }
            "--output-format" => {
                if cli_args.get(index + 1).map(String::as_str) == Some("stream-json") {
                    return false;
                }
                index += 1;
            }
            "--tools" | "--allowedTools" | "--allowed-tools" => {
                let (values, next_index) = collect_variadic_option_values(cli_args, index);
                if !uses_only_native_local_tools(&values) || contains_bash_rule_suffix(&values) {
                    return false;
                }
                saw_tool_restriction = true;
                index = next_index;
            }
            _ if arg.starts_with('-') => return false,
            _ => {}
        }

        index += 1;
    }

    saw_tool_restriction
}

fn is_standalone_help_flag(args: &[String]) -> bool {
    matches!(args, [flag] if flag == "-h" || flag == "--help")
}

fn is_standalone_version_flag(args: &[String]) -> bool {
    matches!(args, [flag] if flag == "-v" || flag == "-V" || flag == "--version")
}

fn determine_route(args: &[String]) -> Route {
    if is_standalone_version_flag(args) {
        return Route::Version;
    }

    if is_standalone_help_flag(args) {
        return Route::Help;
    }

    if should_use_recovery_entrypoint(args) {
        return Route::Recovery;
    }

    if should_use_native_local_tools_entrypoint(args) {
        return Route::NativeLocalTools;
    }

    if should_use_lightweight_headless_print_entrypoint(args) {
        return Route::LightweightHeadless;
    }

    Route::FullCli
}

fn looks_like_repo_root(path: &Path) -> bool {
    path.join("bin/saicode").is_file()
        && path.join("native/saicode-launcher/Cargo.toml").is_file()
        && path.join("rust/Cargo.toml").is_file()
}

fn find_repo_root() -> Result<PathBuf, String> {
    if let Ok(explicit) = env::var("SAICODE_REPO_ROOT") {
        let explicit_path = PathBuf::from(explicit);
        if looks_like_repo_root(&explicit_path) {
            return Ok(explicit_path);
        }
    }

    if let Ok(exe_path) = env::current_exe() {
        for ancestor in exe_path.ancestors() {
            if looks_like_repo_root(ancestor) {
                return Ok(ancestor.to_path_buf());
            }
        }
    }

    if let Ok(cwd) = env::current_dir() {
        for ancestor in cwd.ancestors() {
            if looks_like_repo_root(ancestor) {
                return Ok(ancestor.to_path_buf());
            }
        }
    }

    Err("Could not locate saicode repo root for native launcher".to_string())
}

fn print_help() {
    print!("{FAST_HELP_TEXT}");
}

fn print_version() {
    let version = option_env!("SAICODE_PACKAGE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
    println!("{version} (saicode)");
}

fn current_version() -> &'static str {
    option_env!("SAICODE_PACKAGE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn trace_route(route: Route, target: Option<&Path>) {
    if !is_env_truthy(env::var("SAICODE_NATIVE_TRACE").ok().as_deref()) {
        return;
    }

    match target {
        Some(path) => eprintln!(
            "saicode-native route={} target={}",
            route.label(),
            path.display()
        ),
        None => eprintln!("saicode-native route={}", route.label()),
    }
}

fn maybe_print_dry_run(route: Route, target: Option<&Path>) -> bool {
    if !is_env_truthy(env::var("SAICODE_NATIVE_DRY_RUN").ok().as_deref()) {
        return false;
    }

    match target {
        Some(path) => println!("route={} target={}", route.label(), path.display()),
        None => println!("route={}", route.label()),
    }

    true
}

fn rust_one_shot_binary(repo_root: &Path) -> Option<PathBuf> {
    if is_env_truthy(env::var("SAICODE_DISABLE_RUST_ONE_SHOT").ok().as_deref()) {
        return None;
    }

    let binary = env::var("SAICODE_RUST_ONE_SHOT_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root.join("rust/target/release/saicode-rust-one-shot"));

    binary.is_file().then_some(binary)
}

fn rust_local_tools_binary(repo_root: &Path) -> Option<PathBuf> {
    if is_env_truthy(env::var("SAICODE_DISABLE_RUST_LOCAL_TOOLS").ok().as_deref()) {
        return None;
    }

    let binary = env::var("SAICODE_RUST_LOCAL_TOOLS_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root.join("rust/target/release/saicode-rust-local-tools"));

    binary.is_file().then_some(binary)
}

fn rust_full_cli_binary(repo_root: &Path) -> Option<PathBuf> {
    if is_env_truthy(env::var("SAICODE_DISABLE_RUST_FULL_CLI").ok().as_deref()) {
        return None;
    }

    let binary = env::var("SAICODE_RUST_FULL_CLI_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root.join("rust/target/release/saicode-rust-cli"));

    binary.is_file().then_some(binary)
}

fn rust_full_cli_target(repo_root: &Path) -> Result<PathBuf, String> {
    rust_full_cli_binary(repo_root).ok_or_else(|| {
        format!(
            "Rust full CLI binary not found: {}",
            repo_root.join("rust/target/release/saicode-rust-cli").display()
        )
    })
}

fn run_external_binary(route: Route, target: &Path, args: &[String]) -> Result<(), String> {
    trace_route(route, Some(target));
    if maybe_print_dry_run(route, Some(target)) {
        return Ok(());
    }

    let status = Command::new(target)
        .args(args)
        .env("SAICODE_NATIVE_LAUNCHER", "1")
        .status()
        .map_err(|error| format!("Failed to spawn {}: {error}", target.display()))?;

    process::exit(status.code().unwrap_or(1));
}

enum RustLocalToolsBinaryOutcome {
    Completed,
    FallbackToFullCli(String),
}

enum RustOneShotOutcome {
    Completed,
    FallbackToNativeRecovery(String),
}

fn run_rust_local_tools_binary(
    route: Route,
    target: &Path,
    args: &[String],
) -> Result<RustLocalToolsBinaryOutcome, String> {
    trace_route(route, Some(target));
    if maybe_print_dry_run(route, Some(target)) {
        return Ok(RustLocalToolsBinaryOutcome::Completed);
    }

    let output = Command::new(target)
        .args(args)
        .env("SAICODE_NATIVE_LAUNCHER", "1")
        .output()
        .map_err(|error| format!("Failed to spawn {}: {error}", target.display()))?;

    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    match output.status.code() {
        Some(0) => Ok(RustLocalToolsBinaryOutcome::Completed),
        Some(90) => Ok(RustLocalToolsBinaryOutcome::FallbackToFullCli(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        )),
        Some(91) => Err(format!(
            "{} reported it cannot handle this invocation natively",
            target.display()
        )),
        Some(code) => Err(format!("{} exited with status code {code}", target.display())),
        None => Err(format!("{} exited due to signal", target.display())),
    }
}

fn run_via_rust_full_cli(route: Route, repo_root: &Path, args: &[String]) -> Result<(), String> {
    let target = rust_full_cli_target(repo_root)?;
    run_external_binary(route, &target, args)
}

fn run_rust_one_shot_binary(
    route: Route,
    target: &Path,
    args: &[String],
) -> Result<RustOneShotOutcome, String> {
    trace_route(route, Some(target));
    if maybe_print_dry_run(route, Some(target)) {
        return Ok(RustOneShotOutcome::Completed);
    }

    let output = Command::new(target)
        .args(args)
        .env("SAICODE_NATIVE_LAUNCHER", "1")
        .output()
        .map_err(|error| format!("Failed to spawn {}: {error}", target.display()))?;

    if output.status.success() {
        if !output.stdout.is_empty() {
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        return Ok(RustOneShotOutcome::Completed);
    }

    let mut reason = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if reason.is_empty() {
        reason = match output.status.code() {
            Some(code) => format!("{} exited with status code {code}", target.display()),
            None => format!("{} exited due to signal", target.display()),
        };
    }

    Ok(RustOneShotOutcome::FallbackToNativeRecovery(reason))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let route = determine_route(&args);

    match route {
        Route::Help => {
            trace_route(route, None);
            if !maybe_print_dry_run(route, None) {
                print_help();
            }
        }
        Route::Version => {
            trace_route(route, None);
            if !maybe_print_dry_run(route, None) {
                print_version();
            }
        }
        Route::Recovery => {
            let repo_root = match find_repo_root() {
                Ok(path) => path,
                Err(message) => {
                    eprintln!("{message}");
                    process::exit(1);
                }
            };

            if let Some(target) = rust_one_shot_binary(&repo_root) {
                match run_rust_one_shot_binary(route, &target, &args) {
                    Ok(RustOneShotOutcome::Completed) => return,
                    Ok(RustOneShotOutcome::FallbackToNativeRecovery(reason)) => {
                        if is_env_truthy(env::var("SAICODE_NATIVE_TRACE").ok().as_deref()) {
                            eprintln!(
                                "saicode-native rust-one-shot fallback_to=native-recovery reason={reason}"
                            );
                        }
                    }
                    Err(message) => {
                        if is_env_truthy(env::var("SAICODE_NATIVE_TRACE").ok().as_deref()) {
                            eprintln!(
                                "saicode-native rust-one-shot fallback_to=native-recovery reason={message}"
                            );
                        }
                    }
                }
            }

            if recovery::should_handle_natively(&args) {
                trace_route(route, None);
                if is_env_truthy(env::var("SAICODE_NATIVE_DRY_RUN").ok().as_deref()) {
                    println!("route={} target=native-recovery", route.label());
                    return;
                }
                if let Err(message) = recovery::run_native_recovery(&args, current_version()) {
                    eprintln!("{message}");
                    process::exit(1);
                }
                return;
            }

            if let Err(message) = run_via_rust_full_cli(route, &repo_root, &args) {
                eprintln!("{message}");
                process::exit(1);
            }
        }
        Route::NativeLocalTools => {
            let repo_root = match find_repo_root() {
                Ok(path) => path,
                Err(message) => {
                    eprintln!("{message}");
                    process::exit(1);
                }
            };

            if let Some(target) = rust_local_tools_binary(&repo_root) {
                match run_rust_local_tools_binary(route, &target, &args) {
                    Ok(RustLocalToolsBinaryOutcome::Completed) => return,
                    Ok(RustLocalToolsBinaryOutcome::FallbackToFullCli(reason)) => {
                        if is_env_truthy(env::var("SAICODE_NATIVE_TRACE").ok().as_deref()) {
                            eprintln!(
                                "saicode-native rust-local-tools fallback_to=rust-full-cli reason={reason}"
                            );
                        }
                    }
                    Err(message) => {
                        eprintln!("{message}");
                        process::exit(1);
                    }
                }
            }

            if local_tools::should_handle_natively(&args) {
                trace_route(route, None);
                if is_env_truthy(env::var("SAICODE_NATIVE_DRY_RUN").ok().as_deref()) {
                    println!("route={} target=native-local-tools", route.label());
                    return;
                }
                match local_tools::run_native_local_tools(&args) {
                    Ok(local_tools::NativeLocalToolsOutcome::Completed) => return,
                    Ok(local_tools::NativeLocalToolsOutcome::FallbackToRustFullCli(reason)) => {
                        if is_env_truthy(env::var("SAICODE_NATIVE_TRACE").ok().as_deref()) {
                            eprintln!(
                                "saicode-native native-local-tools fallback_to=rust-full-cli reason={reason}"
                            );
                        }
                    }
                    Err(message) => {
                        eprintln!("{message}");
                        process::exit(1);
                    }
                }
            }

            if let Err(message) = run_via_rust_full_cli(route, &repo_root, &args) {
                eprintln!("{message}");
                process::exit(1);
            }
        }
        Route::LightweightHeadless => {
            let repo_root = match find_repo_root() {
                Ok(path) => path,
                Err(message) => {
                    eprintln!("{message}");
                    process::exit(1);
                }
            };

            if let Err(message) = run_via_rust_full_cli(route, &repo_root, &args) {
                eprintln!("{message}");
                process::exit(1);
            }
        }
        Route::FullCli => {
            let repo_root = match find_repo_root() {
                Ok(path) => path,
                Err(message) => {
                    eprintln!("{message}");
                    process::exit(1);
                }
            };
            if let Some(target) = rust_full_cli_binary(&repo_root) {
                if let Err(message) = run_external_binary(route, &target, &args) {
                    eprintln!("{message}");
                    process::exit(1);
                }
                return;
            }
            eprintln!(
                "Rust full CLI binary not found: {}",
                repo_root.join("rust/target/release/saicode-rust-cli").display()
            );
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        contains_bash_rule_suffix, determine_route, normalize_tool_restriction_values,
        should_use_lightweight_headless_print_entrypoint, should_use_native_local_tools_entrypoint,
        should_use_recovery_entrypoint, uses_only_lightweight_headless_tools,
        uses_only_native_local_tools, Route,
    };

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn normalizes_tool_values_like_ts_router() {
        let values = normalize_tool_restriction_values(&args(&["Read,Grep", "WebSearch(extra)"]));
        assert_eq!(values, vec!["Read", "Grep", "WebSearch"]);
    }

    #[test]
    fn detects_bash_rule_suffixes() {
        assert!(contains_bash_rule_suffix(&args(&["Bash(git:*)"])));
        assert!(!contains_bash_rule_suffix(&args(&["Bash", "Read"])));
    }

    #[test]
    fn keeps_recovery_for_simple_print_requests() {
        assert!(should_use_recovery_entrypoint(&args(&["-p", "hello"])));
        assert!(should_use_recovery_entrypoint(&args(&[
            "-p",
            "--output-format",
            "json",
            "hello",
        ])));
    }

    #[test]
    fn keeps_streaming_and_tools_off_recovery_path() {
        assert!(!should_use_recovery_entrypoint(&args(&[
            "-p",
            "--output-format",
            "stream-json",
            "hello",
        ])));
        assert!(!should_use_recovery_entrypoint(&args(&["-p", "--tools", "Read", "hello"])));
        assert!(!should_use_recovery_entrypoint(&args(&[
            "-p",
            "Use",
            "Read",
            "to",
            "inspect",
            "package.json",
        ])));
    }

    #[test]
    fn lightweight_route_accepts_web_tools_but_rejects_resume() {
        assert!(uses_only_lightweight_headless_tools(&args(&["Read,Grep"])));
        assert!(uses_only_native_local_tools(&args(&["Read,Grep"])));
        assert!(uses_only_native_local_tools(&args(&["Write,Edit"])));
        assert!(uses_only_native_local_tools(&args(&["WebSearch,WebFetch"])));
        assert!(should_use_lightweight_headless_print_entrypoint(&args(&[
            "-p",
            "hello",
            "--tools",
            "WebSearch",
        ])));
        assert!(should_use_native_local_tools_entrypoint(&args(&[
            "-p",
            "hello",
            "--allowedTools",
            "Bash",
        ])));
        assert!(should_use_native_local_tools_entrypoint(&args(&[
            "-p",
            "hello",
            "--allowedTools",
            "Bash",
            "--dangerously-skip-permissions",
        ])));
        assert!(should_use_native_local_tools_entrypoint(&args(&[
            "-p",
            "hello",
            "--allowedTools",
            "Bash",
            "--permission-mode",
            "bypassPermissions",
        ])));
        assert!(!should_use_native_local_tools_entrypoint(&args(&[
            "-p",
            "hello",
            "--allowedTools",
            "Bash(git:*)",
        ])));
        assert!(should_use_native_local_tools_entrypoint(&args(&[
            "-p",
            "hello",
            "--allowedTools",
            "Read",
            "--permission-mode",
            "bypassPermissions",
        ])));
        assert!(!should_use_lightweight_headless_print_entrypoint(&args(&[
            "-p",
            "hello",
            "--allowedTools",
            "Read",
            "--resume",
            "session-id",
        ])));
    }

    #[test]
    fn determines_top_level_route_order() {
        assert_eq!(determine_route(&args(&["--help"])), Route::Help);
        assert_eq!(determine_route(&args(&["--version"])), Route::Version);
        assert_eq!(determine_route(&args(&["-p", "hello"])), Route::Recovery);
        assert_eq!(
            determine_route(&args(&["-p", "hello", "--allowedTools", "Read"])),
            Route::NativeLocalTools
        );
        assert_eq!(
            determine_route(&args(&["-p", "--bare", "hello", "--allowedTools", "Read"])),
            Route::NativeLocalTools
        );
        assert_eq!(
            determine_route(&args(&["-p", "hello", "--allowedTools", "Bash"])),
            Route::NativeLocalTools
        );
        assert_eq!(
            determine_route(&args(&[
                "-p",
                "hello",
                "--allowedTools",
                "Bash",
                "--dangerously-skip-permissions",
            ])),
            Route::NativeLocalTools
        );
        assert_eq!(
            determine_route(&args(&["-p", "hello", "--allowedTools", "Bash(git:*)"])),
            Route::LightweightHeadless
        );
        assert_eq!(
            determine_route(&args(&["-p", "hello", "--tools", "WebFetch"])),
            Route::NativeLocalTools
        );
        assert_eq!(
            determine_route(&args(&["-p", "hello", "--tools", "WebSearch"])),
            Route::NativeLocalTools
        );
        assert_eq!(
            determine_route(&args(&["-p", "hello", "--tools", "Write"])),
            Route::NativeLocalTools
        );
        assert_eq!(
            determine_route(&args(&["-p", "hello", "--allowedTools", "Edit"])),
            Route::NativeLocalTools
        );
        assert_eq!(
            determine_route(&args(&["-p", "hello", "--resume", "session-id"])),
            Route::FullCli
        );
        assert_eq!(determine_route(&args(&["-p", "--", "hello"])), Route::Recovery);
        assert_eq!(
            determine_route(&args(&["-p", "--allowedTools", "Read", "--", "hello"])),
            Route::NativeLocalTools
        );
    }
}
