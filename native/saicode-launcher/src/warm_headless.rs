use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{IsTerminal, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const WARM_MANAGER_START_TIMEOUT: Duration = Duration::from_secs(10);
const WARM_MANAGER_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Eq, PartialEq)]
pub enum WarmHeadlessOutcome {
    Handled(i32),
    Fallback(String),
}

#[derive(Serialize)]
struct WarmManagerRequest {
    argv: Vec<String>,
    cwd: String,
    #[serde(rename = "envFingerprint")]
    env_fingerprint: String,
}

#[derive(Deserialize)]
struct WarmManagerResponse {
    ok: bool,
    #[serde(rename = "exitCode")]
    exit_code: Option<i32>,
    stdout: Option<String>,
    stderr: Option<String>,
    #[serde(rename = "fallbackReason")]
    fallback_reason: Option<String>,
    #[serde(rename = "restartRequired")]
    restart_required: Option<bool>,
}

pub fn should_attempt_warm_headless(args: &[String]) -> bool {
    if is_env_truthy(env::var("SAICODE_DISABLE_WARM_HEADLESS").ok().as_deref()) {
        return false;
    }

    let force_warm = is_env_truthy(env::var("SAICODE_FORCE_WARM_HEADLESS").ok().as_deref());
    if !force_warm && !std::io::stdin().is_terminal() {
        return false;
    }

    has_prompt_positional(args)
}

pub fn run_via_warm_headless(repo_root: &Path, args: &[String]) -> Result<WarmHeadlessOutcome, String> {
    let cwd = env::current_dir().map_err(|error| format!("Failed to read cwd for warm headless path: {error}"))?;
    let socket_path = warm_socket_path();
    let env_fingerprint = compute_env_fingerprint();
    let request = WarmManagerRequest {
        argv: args.to_vec(),
        cwd: cwd.display().to_string(),
        env_fingerprint: env_fingerprint.clone(),
    };

    let mut did_restart = false;

    loop {
        match send_request(&socket_path, &request) {
            Ok(response) => {
                if response.restart_required.unwrap_or(false) && !did_restart {
                    did_restart = true;
                    stop_manager(&socket_path);
                    start_manager(repo_root, &socket_path, &cwd, &env_fingerprint)?;
                    continue;
                }

                if !response.ok {
                    let reason = response
                        .fallback_reason
                        .unwrap_or_else(|| "warm headless manager declined request".to_string());
                    if is_env_truthy(env::var("SAICODE_NATIVE_TRACE").ok().as_deref()) {
                        eprintln!("saicode-native warm-headless fallback_reason={reason}");
                    }
                    return Ok(WarmHeadlessOutcome::Fallback(reason));
                }

                if let Some(stderr) = response.stderr {
                    eprint!("{stderr}");
                }
                if let Some(stdout) = response.stdout {
                    print!("{stdout}");
                }

                return Ok(WarmHeadlessOutcome::Handled(response.exit_code.unwrap_or(1)));
            }
            Err(error) => {
                if did_restart {
                    return Ok(WarmHeadlessOutcome::Fallback(error));
                }

                did_restart = true;
                stop_manager(&socket_path);
                start_manager(repo_root, &socket_path, &cwd, &env_fingerprint)?;
            }
        }
    }
}

fn send_request(socket_path: &Path, request: &WarmManagerRequest) -> Result<WarmManagerResponse, String> {
    let mut stream = UnixStream::connect(socket_path)
        .map_err(|error| format!("Failed to connect warm headless manager at {}: {error}", socket_path.display()))?;
    let payload = serde_json::to_vec(request)
        .map_err(|error| format!("Failed to encode warm headless request: {error}"))?;
    stream
        .write_all(&payload)
        .map_err(|error| format!("Failed to write warm headless request: {error}"))?;
    stream
        .write_all(b"\n")
        .map_err(|error| format!("Failed to write warm headless request delimiter: {error}"))?;
    stream
        .flush()
        .map_err(|error| format!("Failed to flush warm headless request: {error}"))?;

    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .map_err(|error| format!("Failed to read warm headless response: {error}"))?;

    serde_json::from_str::<WarmManagerResponse>(&raw)
        .map_err(|error| format!("Failed to parse warm headless response: {error}"))
}

fn start_manager(
    repo_root: &Path,
    socket_path: &Path,
    cwd: &Path,
    env_fingerprint: &str,
) -> Result<(), String> {
    let warm_entrypoint = repo_root.join("src/entrypoints/headlessPrintWarmWorker.ts");
    let preload = repo_root.join("preload.ts");
    if !warm_entrypoint.is_file() {
        return Err(format!(
            "Warm headless entrypoint not found: {}",
            warm_entrypoint.display()
        ));
    }
    if !preload.is_file() {
        return Err(format!(
            "Warm headless preload not found: {}",
            preload.display()
        ));
    }

    stop_manager(socket_path);

    let bun = env::var("SAICODE_BUN_BIN").unwrap_or_else(|_| "bun".to_string());
    let mut command = Command::new(bun);
    command.arg("--preload");
    command.arg(&preload);
    command.arg(&warm_entrypoint);
    command.arg("--manager");
    command.arg(socket_path);
    command.current_dir(cwd);
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());
    command.env("SAICODE_WARM_ENV_FINGERPRINT", env_fingerprint);
    command.env("SAICODE_NATIVE_LAUNCHER", "1");

    command
        .spawn()
        .map_err(|error| format!("Failed to start warm headless manager: {error}"))?;

    wait_for_socket(socket_path)
}

fn wait_for_socket(socket_path: &Path) -> Result<(), String> {
    let deadline = Instant::now() + WARM_MANAGER_START_TIMEOUT;

    while Instant::now() < deadline {
        if socket_path.exists() {
            match UnixStream::connect(socket_path) {
                Ok(stream) => {
                    drop(stream);
                    return Ok(());
                }
                Err(_) => {}
            }
        }

        thread::sleep(WARM_MANAGER_POLL_INTERVAL);
    }

    Err(format!(
        "Timed out waiting for warm headless manager socket at {}",
        socket_path.display()
    ))
}

fn stop_manager(socket_path: &Path) {
    if socket_path.exists() {
        let _ = fs::remove_file(socket_path);
    }
}

fn warm_socket_path() -> PathBuf {
    let user = env::var("USER").unwrap_or_else(|_| "saicode".to_string());
    env::temp_dir().join(format!("saicode-headless-warm-{user}.sock"))
}

fn compute_env_fingerprint() -> String {
    let mut pairs: Vec<(String, String)> = env::vars()
        .filter(|(key, _)| should_include_env_key(key))
        .collect();
    pairs.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = DefaultHasher::new();
    for (key, value) in pairs {
        key.hash(&mut hasher);
        value.hash(&mut hasher);
    }

    format!("{:016x}", hasher.finish())
}

fn should_include_env_key(key: &str) -> bool {
    !matches!(
        key,
        "_"
            | "OLDPWD"
            | "PWD"
            | "SHLVL"
            | "SAICODE_NATIVE_DRY_RUN"
            | "SAICODE_NATIVE_TRACE"
            | "SAICODE_ROUTED_ENTRYPOINT"
            | "SAICODE_WARM_ENV_FINGERPRINT"
    )
}

fn has_prompt_positional(args: &[String]) -> bool {
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "-p"
            | "--print"
            | "--bare"
            | "--dangerously-skip-permissions"
            | "--allow-dangerously-skip-permissions" => {}
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
            | "-n"
            | "--output-format" => {
                index += 1;
            }
            "--tools" | "--allowedTools" | "--allowed-tools" => {
                let (_, next_index) = collect_variadic_option_values(args, index);
                index = next_index;
            }
            _ if arg.starts_with('-') => return false,
            _ => return true,
        }

        index += 1;
    }

    false
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

fn is_env_truthy(value: Option<&str>) -> bool {
    matches!(
        value.map(|item| item.trim().to_ascii_lowercase()),
        Some(ref item) if matches!(item.as_str(), "1" | "true" | "yes" | "on")
    )
}

#[cfg(test)]
mod tests {
    use super::{compute_env_fingerprint, has_prompt_positional};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn detects_prompt_positionals_without_confusing_flag_values() {
        assert!(has_prompt_positional(&args(&["-p", "hello"])));
        assert!(has_prompt_positional(&args(&[
            "-p",
            "--model",
            "cpa/gpt-5.4",
            "hello"
        ])));
        assert!(!has_prompt_positional(&args(&[
            "-p",
            "--model",
            "cpa/gpt-5.4"
        ])));
        assert!(!has_prompt_positional(&args(&["-p", "--tools", "Read"])));
    }

    #[test]
    fn env_fingerprint_is_stable_without_runtime_only_keys() {
        let first = compute_env_fingerprint();
        let second = compute_env_fingerprint();
        assert_eq!(first, second);
    }
}
