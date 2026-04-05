mod local_tools;
mod recovery;
mod warm_headless;

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

const FAST_HELP_TEXT: &str = r#"Usage: saicode [options] [command] [prompt]

saicode - starts an interactive session by default, use -p/--print for
non-interactive output

Arguments:
  prompt                                            Your prompt

Options:
  --add-dir <directories...>                        Additional directories to allow tool access to
  --agent <agent>                                   Agent for the current session. Overrides the 'agent' setting.
  --agents <json>                                   JSON object defining custom agents (e.g. '{"reviewer": {"description": "Reviews code", "prompt": "You are a code reviewer"}}')
  --allow-dangerously-skip-permissions              Allow Full Access to appear as a selectable option without enabling it by default. Recommended only for sandboxes with no internet access.
  --allowedTools, --allowed-tools <tools...>        Comma or space-separated list of tool names to allow
  --append-system-prompt <prompt>                   Append a system prompt to the default system prompt
  --bare                                            Minimal mode: skip hooks, LSP, plugin sync, attribution, auto-memory, background prefetches, keychain reads, and SAICODE.md auto-discovery.
  --betas <betas...>                                Beta headers to include in API requests (API key users only)
  -c, --continue                                    Continue the most recent conversation in the current directory
  --dangerously-skip-permissions                    Enable Full Access mode
  -d, --debug [filter]                              Enable debug mode with optional category filtering
  --disallowedTools, --disallowed-tools <tools...>  Comma or space-separated list of tool names to deny
  --effort <level>                                  Effort level for the current session
  --fallback-model <model>                          Enable fallback model for --print
  --file <specs...>                                 File resources to download at startup
  --fork-session                                    When resuming, create a new session ID instead of reusing the original
  --from-pr [value]                                 Resume a session linked to a PR by PR number/URL
  -h, --help                                        Display help for command
  --ide                                             Automatically connect to IDE on startup if exactly one valid IDE is available
  --include-hook-events                             Include all hook lifecycle events in the output stream
  --include-partial-messages                        Include partial chunks for print streaming
  --input-format <format>                           Input format for --print
  --json-schema <schema>                            JSON Schema for structured output validation
  --max-budget-usd <amount>                         Maximum API budget for --print
  --mcp-config <configs...>                         Load MCP servers from JSON files or strings
  --mcp-debug                                       [DEPRECATED. Use --debug instead] Enable MCP debug mode
  --model <model>                                   Model for the current session
  -n, --name <name>                                 Set a display name for this session
  --no-session-persistence                          Disable session persistence for --print
  --output-format <format>                          Output format for --print
  --permission-mode <mode>                          Permission mode for the session
  --plugin-dir <path>                               Load plugins from a directory for this session only
  -p, --print                                       Print response and exit
  --replay-user-messages                            Re-emit stdin user messages on stdout for acknowledgment
  -r, --resume [value]                              Resume a conversation by session ID or picker
  --session-id <uuid>                               Use a specific session ID
  --setting-sources <sources>                       Comma-separated list of setting sources to load
  --settings <file-or-json>                         Load additional settings
  --strict-mcp-config                               Only use MCP servers from --mcp-config
  --system-prompt <prompt>                          System prompt override
  --tmux                                            Create a tmux session for the worktree
  --tools <tools...>                                Specify built-in tools to expose
  --verbose                                         Override verbose mode setting from config
  -v, --version                                     Output the version number
  -w, --worktree [name]                             Create a new git worktree for this session

Commands:
  agents [options]                                  List configured agents
  mcp                                               Configure and manage MCP servers
  plugin|plugins                                    Manage saicode plugins
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

    fn entrypoint(self) -> Option<&'static str> {
        match self {
            Route::Help | Route::Version => None,
            Route::Recovery => Some("src/localRecoveryCli.ts"),
            Route::NativeLocalTools => Some("src/entrypoints/headlessPrint.ts"),
            Route::LightweightHeadless => Some("src/entrypoints/headlessPrint.ts"),
            Route::FullCli => Some("src/entrypoints/cli.tsx"),
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

    true
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
    path.join("package.json").is_file() && path.join("src/entrypoints/router.ts").is_file()
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

fn trace_virtual_target(route: Route, target: &str) {
    if !is_env_truthy(env::var("SAICODE_NATIVE_TRACE").ok().as_deref()) {
        return;
    }

    eprintln!("saicode-native route={} target={target}", route.label());
}

fn maybe_print_virtual_dry_run(route: Route, target: &str) -> bool {
    if !is_env_truthy(env::var("SAICODE_NATIVE_DRY_RUN").ok().as_deref()) {
        return false;
    }

    println!("route={} target={target}", route.label());
    true
}

fn hand_off_to_bun(route: Route, repo_root: &Path, args: &[String]) -> Result<(), String> {
    let relative_target = route
        .entrypoint()
        .ok_or_else(|| "Route has no Bun entrypoint".to_string())?;
    let target = repo_root.join(relative_target);
    let preload = repo_root.join("preload.ts");

    if !target.is_file() {
        return Err(format!(
            "Native launcher target not found for route {}: {}",
            route.label(),
            target.display()
        ));
    }

    if !preload.is_file() {
        return Err(format!(
            "Native launcher preload not found for route {}: {}",
            route.label(),
            preload.display()
        ));
    }

    trace_route(route, Some(&target));
    if maybe_print_dry_run(route, Some(&target)) {
        return Ok(());
    }

    let bun = env::var("SAICODE_BUN_BIN").unwrap_or_else(|_| "bun".to_string());
    let mut command = Command::new(bun);
    command.arg("--preload");
    command.arg(&preload);
    command.arg(&target);
    command.args(args);
    command.env("SAICODE_NATIVE_LAUNCHER", "1");
    command.env("SAICODE_ROUTED_ENTRYPOINT", route.label());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = command.exec();
        Err(format!("Failed to exec Bun for {}: {error}", target.display()))
    }

    #[cfg(not(unix))]
    {
        let status = command
            .status()
            .map_err(|error| format!("Failed to spawn Bun for {}: {error}", target.display()))?;
        process::exit(status.code().unwrap_or(1));
    }
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

            if let Err(message) = hand_off_to_bun(route, &repo_root, &args) {
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

            if local_tools::should_handle_natively(&args) {
                trace_route(route, None);
                if is_env_truthy(env::var("SAICODE_NATIVE_DRY_RUN").ok().as_deref()) {
                    println!("route={} target=native-local-tools", route.label());
                    return;
                }
                match local_tools::run_native_local_tools(&args) {
                    Ok(local_tools::NativeLocalToolsOutcome::Completed) => return,
                    Ok(local_tools::NativeLocalToolsOutcome::FallbackToBun(_reason)) => {
                        if let Err(message) = hand_off_to_bun(route, &repo_root, &args) {
                            eprintln!("{message}");
                            process::exit(1);
                        }
                        return;
                    }
                    Err(message) => {
                        eprintln!("{message}");
                        process::exit(1);
                    }
                }
            }

            if let Err(message) = hand_off_to_bun(route, &repo_root, &args) {
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

            if warm_headless::should_attempt_warm_headless(&args) {
                trace_virtual_target(route, "warm-headless-worker");
                if maybe_print_virtual_dry_run(route, "warm-headless-worker") {
                    return;
                }

                match warm_headless::run_via_warm_headless(&repo_root, &args) {
                    Ok(warm_headless::WarmHeadlessOutcome::Handled(exit_code)) => {
                        process::exit(exit_code);
                    }
                    Ok(warm_headless::WarmHeadlessOutcome::Fallback(reason)) => {
                        if is_env_truthy(env::var("SAICODE_NATIVE_TRACE").ok().as_deref()) {
                            eprintln!("saicode-native warm-headless main_fallback_reason={reason}");
                        }
                    }
                    Err(message) => {
                        eprintln!("{message}");
                    }
                }
            }

            if let Err(message) = hand_off_to_bun(route, &repo_root, &args) {
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
            if let Err(message) = hand_off_to_bun(route, &repo_root, &args) {
                eprintln!("{message}");
                process::exit(1);
            }
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
    }
}
