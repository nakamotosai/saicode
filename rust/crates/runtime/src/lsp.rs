use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::time::timeout;

const LSP_TIMEOUT_MS: u64 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspRequest {
    pub operation: String,
    pub file_path: Option<PathBuf>,
    pub line: Option<u32>,
    pub character: Option<u32>,
    pub query: Option<String>,
}

pub async fn execute_lsp_request(request: LspRequest) -> Result<Value, String> {
    let working_path = request
        .file_path
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let workspace_root = detect_workspace_root(&working_path);
    let mut client = LspClient::spawn(&workspace_root).await?;
    client.initialize(&workspace_root).await?;

    if let Some(path) = request.file_path.as_deref() {
        client.did_open(path).await?;
    }

    let result = match request.operation.as_str() {
        "definition" => {
            client
                .request(
                    "textDocument/definition",
                    json!({
                        "textDocument": { "uri": file_uri(require_file_path(request.file_path.as_deref())?) },
                        "position": position_from_request(&request),
                    }),
                )
                .await?
        }
        "references" => {
            client
                .request(
                    "textDocument/references",
                    json!({
                        "textDocument": { "uri": file_uri(require_file_path(request.file_path.as_deref())?) },
                        "position": position_from_request(&request),
                        "context": { "includeDeclaration": true },
                    }),
                )
                .await?
        }
        "hover" => {
            client
                .request(
                    "textDocument/hover",
                    json!({
                        "textDocument": { "uri": file_uri(require_file_path(request.file_path.as_deref())?) },
                        "position": position_from_request(&request),
                    }),
                )
                .await?
        }
        "document_symbols" => {
            client
                .request(
                    "textDocument/documentSymbol",
                    json!({
                        "textDocument": { "uri": file_uri(require_file_path(request.file_path.as_deref())?) },
                    }),
                )
                .await?
        }
        "workspace_symbols" => {
            let query = request
                .query
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "workspace_symbols requires a non-empty query".to_string())?;
            client
                .request("workspace/symbol", json!({ "query": query }))
                .await?
        }
        other => return Err(format!("unsupported LSP operation: {other}")),
    };

    Ok(json!({
        "operation": request.operation,
        "workspace_root": workspace_root.display().to_string(),
        "file_path": request.file_path.map(|path| path.display().to_string()),
        "line": request.line,
        "character": request.character,
        "query": request.query,
        "result": result,
    }))
}

fn require_file_path(path: Option<&Path>) -> Result<&Path, String> {
    path.ok_or_else(|| "this LSP operation requires `path`".to_string())
}

fn position_from_request(request: &LspRequest) -> Value {
    let line = request.line.unwrap_or(1).saturating_sub(1);
    let character = request.character.unwrap_or(0);
    json!({ "line": line, "character": character })
}

fn detect_workspace_root(start: &Path) -> PathBuf {
    let mut current = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if current.join("Cargo.toml").exists() || current.join(".git").exists() {
            return current;
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return start.to_path_buf(),
        }
    }
}

fn file_uri(path: &Path) -> String {
    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    format!(
        "file://{}",
        absolute.display().to_string().replace(' ', "%20")
    )
}

struct LspClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
    next_id: u64,
}

impl LspClient {
    async fn spawn(workspace_root: &Path) -> Result<Self, String> {
        let mut prepend_path = None;
        let mut command = if let Ok(bin) = std::env::var("SAICODE_LSP_SERVER_RUST") {
            let mut command = Command::new(&bin);
            prepend_path = Path::new(&bin)
                .parent()
                .map(|path| path.display().to_string());
            command.current_dir(workspace_root);
            command
        } else if let Some(bin) = resolve_command_path("rust-analyzer") {
            let mut command = Command::new(&bin);
            prepend_path = Path::new(&bin)
                .parent()
                .map(|path| path.display().to_string());
            command.current_dir(workspace_root);
            command
        } else if let Some(bin) = resolve_rustup_rust_analyzer_path() {
            let mut command = Command::new(&bin);
            prepend_path = Path::new(&bin)
                .parent()
                .map(|path| path.display().to_string());
            command.current_dir(workspace_root);
            command
        } else {
            return Err(
                "failed to start rust-analyzer; install it with `rustup component add rust-analyzer` or set SAICODE_LSP_SERVER_RUST".to_string(),
            );
        };
        if let Some(prefix) = prepend_path {
            let current = std::env::var("PATH").unwrap_or_default();
            command.env("PATH", format!("{prefix}:{current}"));
        }
        if let Ok(toolchain) = std::env::var("RUSTUP_TOOLCHAIN") {
            if !toolchain.trim().is_empty() {
                command.env("RUSTUP_TOOLCHAIN", toolchain);
            }
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|error| {
            format!(
                "failed to start rust-analyzer; install it with `rustup component add rust-analyzer` or set SAICODE_LSP_SERVER_RUST: {error}"
            )
        })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "lsp server missing stdin pipe".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "lsp server missing stdout pipe".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "lsp server missing stderr pipe".to_string())?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            stderr: BufReader::new(stderr),
            next_id: 1,
        })
    }

    async fn initialize(&mut self, workspace_root: &Path) -> Result<(), String> {
        let _ = self
            .request(
                "initialize",
                json!({
                    "processId": std::process::id(),
                    "rootPath": workspace_root.display().to_string(),
                    "rootUri": file_uri(workspace_root),
                    "workspaceFolders": [{
                        "uri": file_uri(workspace_root),
                        "name": workspace_root.file_name().and_then(|name| name.to_str()).unwrap_or("workspace"),
                    }],
                    "capabilities": {},
                    "clientInfo": { "name": "saicode", "version": "1.0.0" },
                }),
            )
            .await?;
        self.notify("initialized", json!({})).await
    }

    async fn did_open(&mut self, path: &Path) -> Result<(), String> {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": file_uri(path),
                    "languageId": language_id_for_path(path),
                    "version": 1,
                    "text": text,
                }
            }),
        )
        .await
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.write_message(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .await?;

        loop {
            let message = self.read_message().await?;
            if message.get("id") == Some(&json!(id)) {
                if let Some(error) = message.get("error") {
                    return Err(format!("LSP {method} failed: {error}"));
                }
                return Ok(message.get("result").cloned().unwrap_or(Value::Null));
            }
        }
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        self.write_message(json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }))
        .await
    }

    async fn write_message(&mut self, payload: Value) -> Result<(), String> {
        let body = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        self.stdin
            .write_all(header.as_bytes())
            .await
            .map_err(io_error)?;
        self.stdin.write_all(&body).await.map_err(io_error)?;
        self.stdin.flush().await.map_err(io_error)
    }

    async fn read_message(&mut self) -> Result<Value, String> {
        timeout(Duration::from_millis(LSP_TIMEOUT_MS), async {
            let content_length = match read_content_length(&mut self.stdout).await {
                Ok(value) => value,
                Err(error) => {
                    let mut stderr = String::new();
                    let _ = self.stderr.read_to_string(&mut stderr).await;
                    let status = self
                        .child
                        .try_wait()
                        .ok()
                        .flatten()
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "running".to_string());
                    return Err(if stderr.trim().is_empty() {
                        format!("{error} (server status: {status})")
                    } else {
                        format!(
                            "{error} (server status: {status}, stderr: {})",
                            stderr.trim()
                        )
                    });
                }
            };
            let mut body = vec![0_u8; content_length];
            self.stdout.read_exact(&mut body).await.map_err(io_error)?;
            serde_json::from_slice::<Value>(&body).map_err(|error| error.to_string())
        })
        .await
        .map_err(|_| "timed out waiting for LSP server response".to_string())?
    }
}

fn resolve_command_path(command: &str) -> Option<String> {
    let output = std::process::Command::new("sh")
        .args(["-lc", &format!("command -v {command}")])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let path = stdout.lines().next()?.trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

fn resolve_rustup_rust_analyzer_path() -> Option<String> {
    let mut command = std::process::Command::new("rustup");
    if let Ok(toolchain) = std::env::var("RUSTUP_TOOLCHAIN") {
        if !toolchain.trim().is_empty() {
            command.args(["which", "--toolchain", toolchain.trim(), "rust-analyzer"]);
        } else {
            command.args(["which", "rust-analyzer"]);
        }
    } else {
        command.args(["which", "rust-analyzer"]);
    }

    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let path = stdout.trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

async fn read_content_length(stdout: &mut BufReader<ChildStdout>) -> Result<usize, String> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let read = stdout.read_line(&mut line).await.map_err(io_error)?;
        if read == 0 {
            return Err("lsp server closed stdout unexpectedly".to_string());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    content_length.ok_or_else(|| "missing Content-Length in LSP response".to_string())
}

fn language_id_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rust",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("py") => "python",
        Some("go") => "go",
        Some("c") | Some("h") => "c",
        Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") => "cpp",
        _ => "plaintext",
    }
}

fn io_error(error: io::Error) -> String {
    error.to_string()
}
