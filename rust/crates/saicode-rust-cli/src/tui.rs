use std::env;
use std::io::{self, BufRead, BufReader, IsTerminal, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use commands::{build_command_registry_snapshot_with_cwd, CommandRegistryContext, CommandSurface};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Position;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use serde_json::{json, Value};

use crate::{CliArgs, ResumeTarget};

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const MODEL_CANDIDATES: &[(&str, &str)] = &[
    ("cpa/qwen/qwen3.5-122b-a10b", "Qwen 3.5 122B"),
    ("cpa/gpt-5.4", "GPT-5.4"),
    ("cpa/gpt-5.4-mini", "GPT-5.4 Mini"),
    ("cpa/qwen/qwen3.5-397b-a17b", "Qwen 3.5 397B"),
    ("cpa/qwen3-coder-plus", "Qwen3 Coder Plus"),
    ("cpa/nvidia/nemotron-3-super-120b-a12b", "Nemotron 120B"),
    ("cpa/openai/gpt-oss-120b", "GPT-OSS 120B"),
    ("cpa/google/gemma-4-31b-it", "Gemma 4 31B IT"),
    (
        "cpa/opencode/qwen3.6-plus-free",
        "OpenCode Qwen 3.6 Plus Free",
    ),
    ("cpa/opencode/mimo-v2-pro-free", "OpenCode MiMo V2 Pro Free"),
    (
        "cpa/opencode/mimo-v2-omni-free",
        "OpenCode MiMo V2 Omni Free",
    ),
    ("cpa/vision-model", "Vision Model"),
];
const COMMON_SLASH_COMMANDS: &[&str] = &[
    "new",
    "model",
    "status",
    "help",
    "permissions",
    "resume",
    "doctor",
    "config",
    "mcp",
    "memory",
    "compact",
];

pub(crate) fn run(args: &CliArgs, cwd: &Path) -> Result<(), String> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(
            "saicode Rust TUI requires an interactive terminal; use -p/--print or explicit commands instead"
                .to_string(),
        );
    }

    let mut backend = BackendBridge::spawn(args, cwd)?;
    let mut app = TuiApp::new(cwd);

    enable_raw_mode().map_err(|error| error.to_string())?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|error| error.to_string())?;
    let backend_ui = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend_ui).map_err(|error| error.to_string())?;
    terminal.show_cursor().map_err(|error| error.to_string())?;

    let result = run_loop(&mut terminal, &mut backend, &mut app);

    disable_raw_mode().map_err(|error| error.to_string())?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|error| error.to_string())?;
    terminal.show_cursor().map_err(|error| error.to_string())?;

    let _ = backend.shutdown();
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    backend: &mut BackendBridge,
    app: &mut TuiApp,
) -> Result<(), String> {
    while !app.should_quit {
        while let Some(event) = backend.try_recv()? {
            app.handle_backend_event(event);
        }

        terminal
            .draw(|frame| draw(frame, app))
            .map_err(|error| error.to_string())?;

        if event::poll(Duration::from_millis(50)).map_err(|error| error.to_string())? {
            match event::read().map_err(|error| error.to_string())? {
                Event::Key(key) => {
                    app.handle_key(key, backend)?;
                }
                Event::Mouse(mouse) => app.handle_mouse(mouse),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        app.tick();
    }
    Ok(())
}

struct BackendBridge {
    child: Child,
    stdin: ChildStdin,
    receiver: Receiver<BackendEvent>,
}

enum BackendEvent {
    Json(Value),
    Stderr(String),
    Exited(Option<i32>),
}

impl BackendBridge {
    fn spawn(args: &CliArgs, cwd: &Path) -> Result<Self, String> {
        let exe = env::current_exe().map_err(|error| error.to_string())?;
        let mut child = Command::new(exe)
            .args(build_bridge_args(args))
            .current_dir(cwd)
            .env("SAICODE_REPO_ROOT", cwd.display().to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("failed to start Rust backend bridge: {error}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "backend bridge stdin unavailable".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "backend bridge stdout unavailable".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "backend bridge stderr unavailable".to_string())?;
        let (sender, receiver) = mpsc::channel::<BackendEvent>();

        {
            let sender = sender.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    match line {
                        Ok(line) => match serde_json::from_str::<Value>(&line) {
                            Ok(value) => {
                                let _ = sender.send(BackendEvent::Json(value));
                            }
                            Err(error) => {
                                let _ = sender.send(BackendEvent::Stderr(format!(
                                    "invalid bridge json: {error}: {line}"
                                )));
                            }
                        },
                        Err(error) => {
                            let _ = sender.send(BackendEvent::Stderr(format!(
                                "backend stdout read failed: {error}"
                            )));
                            break;
                        }
                    }
                }
            });
        }

        {
            let sender = sender.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(line) => {
                            let _ = sender.send(BackendEvent::Stderr(line));
                        }
                        Err(error) => {
                            let _ = sender.send(BackendEvent::Stderr(format!(
                                "backend stderr read failed: {error}"
                            )));
                            break;
                        }
                    }
                }
            });
        }

        Ok(Self {
            child,
            stdin,
            receiver,
        })
    }

    fn try_recv(&mut self) -> Result<Option<BackendEvent>, String> {
        if let Ok(Some(status)) = self.child.try_wait() {
            return Ok(Some(BackendEvent::Exited(status.code())));
        }
        match self.receiver.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Ok(None),
        }
    }

    fn send_json(&mut self, payload: Value) -> Result<(), String> {
        writeln!(
            self.stdin,
            "{}",
            serde_json::to_string(&payload).map_err(|error| error.to_string())?
        )
        .map_err(|error| error.to_string())?;
        self.stdin.flush().map_err(|error| error.to_string())
    }

    fn send_user_turn(&mut self, prompt: &str) -> Result<(), String> {
        self.send_json(json!({ "type": "user_turn", "prompt": prompt }))
    }

    fn send_slash_command(&mut self, input: &str) -> Result<(), String> {
        self.send_json(json!({ "type": "slash_command", "input": input }))
    }

    fn send_permission_response(&mut self, decision: &str) -> Result<(), String> {
        self.send_json(json!({
            "type": "permission_response",
            "decision": decision,
        }))
    }

    fn shutdown(&mut self) -> Result<(), String> {
        let _ = self.send_json(json!({ "type": "shutdown" }));
        let _ = self.child.kill();
        Ok(())
    }
}

fn build_bridge_args(args: &CliArgs) -> Vec<String> {
    let mut output = Vec::new();
    if let Some(model) = &args.model {
        output.push("--model".to_string());
        output.push(model.clone());
    }
    if let Some(effort) = args.effort {
        output.push("--effort".to_string());
        output.push(effort.saicode_wire_value().to_string());
    }
    if let Some(permission_mode) = args.permission_mode {
        output.push("--permission-mode".to_string());
        output.push(permission_mode.as_str().to_string());
    }
    for tool in &args.allowed_tools {
        output.push("--allowed-tools".to_string());
        output.push(tool.clone());
    }
    for tool in &args.disallowed_tools {
        output.push("--disallowed-tools".to_string());
        output.push(tool.clone());
    }
    if let Some(prompt) = &args.system_prompt {
        output.push("--system-prompt".to_string());
        output.push(prompt.clone());
    }
    if let Some(prompt) = &args.append_system_prompt {
        output.push("--append-system-prompt".to_string());
        output.push(prompt.clone());
    }
    if args.no_session_persistence {
        output.push("--no-session-persistence".to_string());
    }
    if args.bare {
        output.push("--bare".to_string());
    }
    if let Some(resume) = &args.resume {
        match resume {
            ResumeTarget::Latest => output.push("--continue".to_string()),
            ResumeTarget::Path(path) => {
                output.push("--resume".to_string());
                output.push(path.display().to_string());
            }
        }
    }
    output.push("ui-bridge".to_string());
    output
}

#[derive(Clone)]
struct MessageItem {
    kind: MessageKind,
    title: String,
    body: String,
    meta: Option<String>,
}

#[derive(Clone, Copy)]
enum MessageKind {
    User,
    Assistant,
    Tool,
    System,
    Error,
}

struct PermissionState {
    tool_name: String,
    current_mode: String,
    required_mode: String,
    reason: Option<String>,
    input: Option<String>,
    allow_selected: bool,
}

struct PickerState {
    query: String,
    items: Vec<ModelCandidate>,
    filtered: Vec<usize>,
    selected: usize,
}

struct SlashPickerState {
    query: String,
    items: Vec<SlashCommandCandidate>,
    filtered: Vec<usize>,
    selected: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MouseMode {
    Scroll,
    Select,
}

impl MouseMode {
    fn label(self) -> &'static str {
        match self {
            Self::Scroll => "scroll",
            Self::Select => "select",
        }
    }
}

#[derive(Clone)]
struct ModelCandidate {
    id: String,
    label: String,
}

#[derive(Clone)]
struct SlashCommandCandidate {
    insert_text: String,
    label: String,
    summary: String,
    aliases: Vec<String>,
    rank: usize,
}

struct TuiApp {
    repo_root: String,
    messages: Vec<MessageItem>,
    draft_assistant: String,
    input: String,
    cursor: usize,
    ready: bool,
    busy: bool,
    should_quit: bool,
    session_model: String,
    session_profile: String,
    session_provider: String,
    session_wire_model: String,
    session_permission: String,
    session_workspace: String,
    usage_input: u64,
    usage_output: u64,
    context_tokens: u64,
    context_window_tokens: u64,
    context_percent: f64,
    context_session_tokens: u64,
    context_system_tokens: u64,
    context_tool_tokens: u64,
    auto_compact_percent: u64,
    session_message_count: u64,
    session_compaction_count: u64,
    history: Vec<String>,
    history_cursor: Option<usize>,
    permission: Option<PermissionState>,
    picker: Option<PickerState>,
    slash_picker: Option<SlashPickerState>,
    scroll: u16,
    follow_latest: bool,
    last_transcript_max_scroll: u16,
    status_lines: Vec<String>,
    mouse_mode: MouseMode,
    spinner_index: usize,
    last_tick: Instant,
}

impl TuiApp {
    fn new(cwd: &Path) -> Self {
        Self {
            repo_root: cwd.display().to_string(),
            messages: Vec::new(),
            draft_assistant: String::new(),
            input: String::new(),
            cursor: 0,
            ready: false,
            busy: false,
            should_quit: false,
            session_model: String::new(),
            session_profile: String::new(),
            session_provider: String::new(),
            session_wire_model: String::new(),
            session_permission: String::new(),
            session_workspace: cwd.display().to_string(),
            usage_input: 0,
            usage_output: 0,
            context_tokens: 0,
            context_window_tokens: 270_000,
            context_percent: 0.0,
            context_session_tokens: 0,
            context_system_tokens: 0,
            context_tool_tokens: 0,
            auto_compact_percent: 80,
            session_message_count: 0,
            session_compaction_count: 0,
            history: Vec::new(),
            history_cursor: None,
            permission: None,
            picker: None,
            slash_picker: None,
            scroll: 0,
            follow_latest: true,
            last_transcript_max_scroll: 0,
            status_lines: vec!["Starting Rust backend…".to_string()],
            mouse_mode: MouseMode::Scroll,
            spinner_index: 0,
            last_tick: Instant::now(),
        }
    }

    fn tick(&mut self) {
        if self.busy && self.last_tick.elapsed() >= Duration::from_millis(80) {
            self.spinner_index = (self.spinner_index + 1) % SPINNER_FRAMES.len();
            self.last_tick = Instant::now();
        }
    }

    fn spinner(&self) -> &'static str {
        SPINNER_FRAMES[self.spinner_index % SPINNER_FRAMES.len()]
    }

    fn push_status(&mut self, status: impl Into<String>) {
        let status = status.into();
        if self.status_lines.last() == Some(&status) {
            return;
        }
        self.status_lines.push(status);
        if self.status_lines.len() > 2 {
            let excess = self.status_lines.len() - 2;
            self.status_lines.drain(0..excess);
        }
    }

    fn add_message(
        &mut self,
        kind: MessageKind,
        title: impl Into<String>,
        body: impl Into<String>,
        meta: Option<String>,
    ) {
        self.messages.push(MessageItem {
            kind,
            title: title.into(),
            body: body.into(),
            meta,
        });
    }

    fn handle_backend_event(&mut self, event: BackendEvent) {
        match event {
            BackendEvent::Stderr(line) => {
                self.push_status("Rust backend stderr.");
                self.add_message(MessageKind::Error, "Rust backend", line, None);
                self.busy = false;
            }
            BackendEvent::Exited(code) => {
                self.push_status("Rust backend exited.");
                self.add_message(
                    MessageKind::Error,
                    "Rust backend exited",
                    format!("exit code: {}", code.unwrap_or_default()),
                    None,
                );
                self.ready = false;
                self.busy = false;
            }
            BackendEvent::Json(value) => self.handle_bridge_json(value),
        }
    }

    fn handle_bridge_json(&mut self, value: Value) {
        let Some(event_type) = value.get("type").and_then(Value::as_str) else {
            return;
        };
        match event_type {
            "session_started" | "session_updated" => {
                self.session_model = string_field(&value, "model");
                self.session_profile = string_field(&value, "profile");
                self.session_provider = string_field(&value, "provider");
                self.session_wire_model = string_field(&value, "wire_model");
                self.session_permission = string_field(&value, "permission_mode");
                self.session_workspace = string_field(&value, "workspace");
                if let Some(usage) = value.get("usage") {
                    self.usage_input = usage
                        .get("input_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(self.usage_input);
                    self.usage_output = usage
                        .get("output_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(self.usage_output);
                }
                self.context_tokens = value
                    .get("context_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(self.context_tokens);
                self.context_window_tokens = value
                    .get("context_window_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(self.context_window_tokens);
                self.context_percent = value
                    .get("context_percent")
                    .and_then(Value::as_f64)
                    .unwrap_or(self.context_percent);
                if let Some(context_breakdown) = value.get("context_breakdown") {
                    self.context_session_tokens = context_breakdown
                        .get("session_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(self.context_session_tokens);
                    self.context_system_tokens = context_breakdown
                        .get("system_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(self.context_system_tokens);
                    self.context_tool_tokens = context_breakdown
                        .get("tool_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(self.context_tool_tokens);
                }
                self.auto_compact_percent = value
                    .get("auto_compact_percent")
                    .and_then(Value::as_u64)
                    .unwrap_or(self.auto_compact_percent);
                if let Some(session) = value.get("session") {
                    self.session_message_count = session
                        .get("message_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(self.session_message_count);
                    self.session_compaction_count = session
                        .get("compaction_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(self.session_compaction_count);
                }
            }
            "ready" => {
                self.ready = true;
                self.push_status("Rust backend ready.");
            }
            "turn_started" => {
                self.busy = true;
                self.draft_assistant.clear();
                self.follow_latest = true;
                self.push_status("Preparing request…");
            }
            "turn_status" => {
                let text = string_field(&value, "text");
                if !text.is_empty() {
                    self.push_status(text);
                }
            }
            "content_delta" => {
                self.busy = true;
                if self.draft_assistant.is_empty() {
                    self.push_status("Receiving response…");
                }
                self.draft_assistant.push_str(
                    value
                        .get("delta")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                );
                self.follow_latest = true;
            }
            "final_message" => {
                self.busy = false;
                self.push_status("Response complete.");
                let text = string_field(&value, "text");
                let model = string_field(&value, "model");
                if let Some(usage) = value.get("usage") {
                    self.usage_input = usage
                        .get("input_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(self.usage_input);
                    self.usage_output = usage
                        .get("output_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(self.usage_output);
                }
                self.draft_assistant.clear();
                self.add_message(MessageKind::Assistant, "Saicode", text, Some(model));
                self.follow_latest = true;
            }
            "tool_start" => {
                self.push_status(format!("Running tool: {}", string_field(&value, "name")));
                self.add_message(
                    MessageKind::Tool,
                    format!("Tool · {}", string_field(&value, "name")),
                    pretty_json(value.get("input")),
                    Some("started".to_string()),
                );
                self.follow_latest = true;
            }
            "tool_result" => {
                self.push_status(format!("Tool completed: {}", string_field(&value, "name")));
                self.add_message(
                    MessageKind::Tool,
                    format!("Tool · {}", string_field(&value, "name")),
                    string_field(&value, "output"),
                    value
                        .get("qualified_name")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                );
                self.follow_latest = true;
            }
            "tool_error" => {
                self.push_status(format!("Tool failed: {}", string_field(&value, "name")));
                self.add_message(
                    MessageKind::Error,
                    format!("Tool · {}", string_field(&value, "name")),
                    string_field(&value, "error"),
                    value
                        .get("qualified_name")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                );
                self.follow_latest = true;
            }
            "permission_request" => {
                self.push_status(format!(
                    "Waiting for permission: {}",
                    string_field(&value, "tool_name")
                ));
                self.permission = Some(PermissionState {
                    tool_name: string_field(&value, "tool_name"),
                    current_mode: string_field(&value, "current_mode"),
                    required_mode: string_field(&value, "required_mode"),
                    reason: value
                        .get("reason")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    input: value.get("input").map(|input| pretty_json(Some(input))),
                    allow_selected: true,
                });
                self.follow_latest = true;
            }
            "permission_resolved" => {
                let tool_name = string_field(&value, "tool_name");
                let decision = string_field(&value, "decision");
                self.permission = None;
                self.add_message(
                    if decision == "allow" {
                        MessageKind::System
                    } else {
                        MessageKind::Error
                    },
                    format!("Permission · {tool_name}"),
                    string_field(&value, "reason"),
                    Some(decision.clone()),
                );
                self.follow_latest = true;
                self.push_status(format!("Permission resolved: {decision}"));
            }
            "slash_result" => {
                let input = string_field(&value, "input");
                let text = string_field(&value, "text");
                self.add_message(MessageKind::System, input, text, None);
                self.busy = false;
                self.follow_latest = true;
                if value
                    .get("should_exit")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    self.should_quit = true;
                }
                self.push_status("Command complete.");
            }
            "usage" => {
                self.usage_input = value
                    .get("input_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(self.usage_input);
                self.usage_output = value
                    .get("output_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(self.usage_output);
            }
            "auto_compaction" => {
                let removed = value
                    .get("removed_message_count")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                self.add_message(
                    MessageKind::System,
                    "Auto compact",
                    format!(
                        "Context reached {}% threshold. Compacted {} older messages.",
                        self.auto_compact_percent, removed
                    ),
                    Some("automatic".to_string()),
                );
                self.follow_latest = true;
                self.push_status("Context auto-compacted.");
            }
            "error" => {
                self.busy = false;
                self.push_status("Request failed.");
                self.add_message(
                    MessageKind::Error,
                    "Rust backend",
                    string_field(&value, "message"),
                    None,
                );
                self.follow_latest = true;
            }
            "shutdown_complete" => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent, backend: &mut BackendBridge) -> Result<(), String> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return Ok(());
        }

        if self.permission.is_some() {
            return self.handle_permission_key(key, backend);
        }

        if self.picker.is_some() {
            return self.handle_picker_key(key, backend);
        }
        if self.slash_picker.is_some() {
            return self.handle_slash_picker_key(key, backend);
        }

        match key.code {
            KeyCode::Char(c)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                self.insert_char(c);
            }
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete_char(),
            KeyCode::Left => {
                self.cursor = previous_char_boundary(&self.input, self.cursor);
            }
            KeyCode::Right => {
                self.cursor = next_char_boundary(&self.input, self.cursor);
            }
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.input.len(),
            KeyCode::Esc => {
                self.input.clear();
                self.cursor = 0;
                self.history_cursor = None;
                self.sync_slash_picker();
            }
            KeyCode::Up => self.history_up(),
            KeyCode::Down => self.history_down(),
            KeyCode::PageUp => self.scroll_up(6),
            KeyCode::PageDown => self.scroll_down(6),
            KeyCode::Enter => self.submit(backend)?,
            KeyCode::F(2) => self.open_model_picker(""),
            KeyCode::F(3) => self.toggle_mouse_mode()?,
            _ => {}
        }
        Ok(())
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.mouse_mode == MouseMode::Select {
            return;
        }
        if self.permission.is_some() || self.picker.is_some() {
            return;
        }
        match mouse.kind {
            MouseEventKind::ScrollUp => self.scroll_up(3),
            MouseEventKind::ScrollDown => self.scroll_down(3),
            _ => {}
        }
    }

    fn scroll_up(&mut self, amount: u16) {
        if self.follow_latest {
            self.scroll = self.last_transcript_max_scroll;
            self.follow_latest = false;
        }
        self.scroll = self.scroll.saturating_sub(amount);
    }

    fn scroll_down(&mut self, amount: u16) {
        if self.follow_latest {
            return;
        }
        self.scroll = self
            .scroll
            .saturating_add(amount)
            .min(self.last_transcript_max_scroll);
        if self.scroll >= self.last_transcript_max_scroll {
            self.follow_latest = true;
        }
    }

    fn toggle_mouse_mode(&mut self) -> Result<(), String> {
        self.mouse_mode = match self.mouse_mode {
            MouseMode::Scroll => {
                execute!(io::stdout(), DisableMouseCapture).map_err(|error| error.to_string())?;
                self.push_status("Mouse mode: select text.");
                MouseMode::Select
            }
            MouseMode::Select => {
                execute!(io::stdout(), EnableMouseCapture).map_err(|error| error.to_string())?;
                self.push_status("Mouse mode: scroll transcript.");
                MouseMode::Scroll
            }
        };
        Ok(())
    }

    fn handle_permission_key(
        &mut self,
        key: KeyEvent,
        backend: &mut BackendBridge,
    ) -> Result<(), String> {
        let Some(permission) = self.permission.as_mut() else {
            return Ok(());
        };
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => permission.allow_selected = true,
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => permission.allow_selected = false,
            KeyCode::Char('y') => {
                backend.send_permission_response("allow")?;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                backend.send_permission_response("deny")?;
            }
            KeyCode::Enter => {
                backend.send_permission_response(if permission.allow_selected {
                    "allow"
                } else {
                    "deny"
                })?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_picker_key(
        &mut self,
        key: KeyEvent,
        backend: &mut BackendBridge,
    ) -> Result<(), String> {
        let Some(picker) = self.picker.as_mut() else {
            return Ok(());
        };
        match key.code {
            KeyCode::Esc => {
                self.picker = None;
            }
            KeyCode::Up => {
                if picker.selected > 0 {
                    picker.selected -= 1;
                }
            }
            KeyCode::Down => {
                if picker.selected + 1 < picker.filtered.len() {
                    picker.selected += 1;
                }
            }
            KeyCode::Backspace => {
                picker.query.pop();
                picker.recompute();
            }
            KeyCode::Enter => {
                if let Some(model) = picker.selected_model() {
                    let command = format!("/model {}", model.id);
                    self.add_message(
                        MessageKind::System,
                        "Command",
                        command.clone(),
                        Some("picker".to_string()),
                    );
                    backend.send_slash_command(&command)?;
                    self.picker = None;
                }
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                picker.query.push(c);
                picker.recompute();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_slash_picker_key(
        &mut self,
        key: KeyEvent,
        backend: &mut BackendBridge,
    ) -> Result<(), String> {
        match key.code {
            KeyCode::Esc => {
                self.slash_picker = None;
            }
            KeyCode::Up => {
                if let Some(picker) = self.slash_picker.as_mut() {
                    if picker.selected > 0 {
                        picker.selected -= 1;
                    }
                }
            }
            KeyCode::Down => {
                if let Some(picker) = self.slash_picker.as_mut() {
                    if picker.selected + 1 < picker.filtered.len() {
                        picker.selected += 1;
                    }
                }
            }
            KeyCode::Tab => {
                self.apply_selected_slash_candidate();
            }
            KeyCode::Enter => {
                if self.should_accept_slash_candidate_on_enter() {
                    self.apply_selected_slash_candidate();
                } else {
                    self.submit(backend)?;
                }
            }
            KeyCode::Backspace => {
                self.backspace();
            }
            KeyCode::Delete => {
                self.delete_char();
            }
            KeyCode::Left => {
                self.cursor = previous_char_boundary(&self.input, self.cursor);
            }
            KeyCode::Right => {
                self.cursor = next_char_boundary(&self.input, self.cursor);
            }
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.input.len(),
            KeyCode::Char(c)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                self.insert_char(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn submit(&mut self, backend: &mut BackendBridge) -> Result<(), String> {
        let prompt = self.input.trim().to_string();
        if prompt.is_empty() {
            return Ok(());
        }

        if self.history.last().is_none() || self.history.last() != Some(&prompt) {
            self.history.push(prompt.clone());
        }
        self.history_cursor = None;
        self.input.clear();
        self.cursor = 0;
        self.sync_slash_picker();

        if prompt == "/model" {
            self.open_model_picker("");
            return Ok(());
        }
        if let Some(filter) = prompt.strip_prefix("/model ") {
            self.open_model_picker(filter);
            return Ok(());
        }

        if prompt.starts_with('/') {
            self.add_message(MessageKind::System, "Command", prompt.clone(), None);
            backend.send_slash_command(&prompt)?;
            return Ok(());
        }

        self.add_message(MessageKind::User, "You", prompt.clone(), None);
        self.busy = true;
        self.draft_assistant.clear();
        self.follow_latest = true;
        backend.send_user_turn(&prompt)
    }

    fn open_model_picker(&mut self, filter: &str) {
        let mut items = MODEL_CANDIDATES
            .iter()
            .map(|(id, label)| ModelCandidate {
                id: (*id).to_string(),
                label: (*label).to_string(),
            })
            .collect::<Vec<_>>();
        if !self.session_model.is_empty()
            && !items
                .iter()
                .any(|candidate| candidate.id == self.session_model)
        {
            items.insert(
                0,
                ModelCandidate {
                    id: self.session_model.clone(),
                    label: "Current model".to_string(),
                },
            );
        }
        let mut picker = PickerState {
            query: filter.trim().to_string(),
            items,
            filtered: Vec::new(),
            selected: 0,
        };
        picker.recompute();
        if let Some(index) = picker
            .filtered
            .iter()
            .position(|index| picker.items[*index].id == self.session_model)
        {
            picker.selected = index;
        }
        self.picker = Some(picker);
    }

    fn insert_char(&mut self, ch: char) {
        self.input.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.sync_slash_picker();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 || self.input.is_empty() {
            return;
        }
        let previous = previous_char_boundary(&self.input, self.cursor);
        self.input.drain(previous..self.cursor);
        self.cursor = previous;
        self.sync_slash_picker();
    }

    fn delete_char(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        let next = next_char_boundary(&self.input, self.cursor);
        self.input.drain(self.cursor..next);
        self.sync_slash_picker();
    }

    fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next = match self.history_cursor {
            Some(current) if current > 0 => current - 1,
            Some(current) => current,
            None => self.history.len() - 1,
        };
        self.history_cursor = Some(next);
        self.input = self.history[next].clone();
        self.cursor = self.input.len();
        self.sync_slash_picker();
    }

    fn history_down(&mut self) {
        let Some(current) = self.history_cursor else {
            return;
        };
        if current + 1 >= self.history.len() {
            self.history_cursor = None;
            self.input.clear();
        } else {
            self.history_cursor = Some(current + 1);
            self.input = self.history[current + 1].clone();
        }
        self.cursor = self.input.len();
        self.sync_slash_picker();
    }

    fn sync_slash_picker(&mut self) {
        if self.input.starts_with('/') && !self.input.contains(' ') {
            let query = self.input.trim().trim_start_matches('/').to_string();
            match self.slash_picker.as_mut() {
                Some(picker) => {
                    picker.query = query;
                    picker.recompute();
                }
                None => {
                    let items = build_slash_command_candidates(Path::new(&self.repo_root));
                    let mut picker = SlashPickerState {
                        query,
                        items,
                        filtered: Vec::new(),
                        selected: 0,
                    };
                    picker.recompute();
                    self.slash_picker = Some(picker);
                }
            }
        } else {
            self.slash_picker = None;
        }
    }

    fn should_accept_slash_candidate_on_enter(&self) -> bool {
        let Some(picker) = self.slash_picker.as_ref() else {
            return false;
        };
        let Some(candidate) = picker.selected_command() else {
            return false;
        };
        let prompt = self.input.trim();
        prompt == "/" || prompt != candidate.insert_text
    }

    fn apply_selected_slash_candidate(&mut self) {
        let Some(picker) = self.slash_picker.as_ref() else {
            return;
        };
        let Some(candidate) = picker.selected_command() else {
            return;
        };
        let needs_space = candidate.insert_text.as_str().eq("/model")
            || candidate.insert_text.as_str().eq("/resume")
            || candidate.insert_text.as_str().eq("/config")
            || candidate.insert_text.as_str().eq("/mcp")
            || candidate.insert_text.as_str().eq("/permissions");
        self.input = if needs_space {
            format!("{} ", candidate.insert_text)
        } else {
            candidate.insert_text.clone()
        };
        self.cursor = self.input.len();
        self.sync_slash_picker();
    }
}

impl PickerState {
    fn recompute(&mut self) {
        let query = self.query.to_ascii_lowercase();
        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                query.is_empty()
                    || item.id.to_ascii_lowercase().contains(&query)
                    || item.label.to_ascii_lowercase().contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
        if self.filtered.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
        }
    }

    fn selected_model(&self) -> Option<&ModelCandidate> {
        let index = *self.filtered.get(self.selected)?;
        self.items.get(index)
    }
}

impl SlashPickerState {
    fn recompute(&mut self) {
        let query = self.query.to_ascii_lowercase();
        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                query.is_empty()
                    || item.label.to_ascii_lowercase().contains(&query)
                    || item.summary.to_ascii_lowercase().contains(&query)
                    || item
                        .aliases
                        .iter()
                        .any(|alias| alias.to_ascii_lowercase().contains(&query))
            })
            .map(|(index, _)| index)
            .collect();
        self.filtered.sort_by_key(|index| {
            let item = &self.items[*index];
            let label = item.label.to_ascii_lowercase();
            let prefix_rank = if query.is_empty() {
                0
            } else if label.starts_with(&format!("/{query}")) {
                0
            } else if item
                .aliases
                .iter()
                .any(|alias| alias.to_ascii_lowercase().starts_with(&query))
            {
                1
            } else {
                2
            };
            (item.rank, prefix_rank, item.label.clone())
        });
        if self.filtered.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
        }
    }

    fn selected_command(&self) -> Option<&SlashCommandCandidate> {
        let index = *self.filtered.get(self.selected)?;
        self.items.get(index)
    }
}

fn draw(frame: &mut Frame<'_>, app: &mut TuiApp) {
    let transcript_lines = build_transcript_lines(app);
    let footer_height = footer_lines(app).len() as u16;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(footer_height),
        ])
        .split(frame.area());

    draw_header(frame, chunks[0], app);
    draw_messages(frame, chunks[1], app, transcript_lines);
    draw_input(frame, chunks[2], app);
    draw_footer(frame, chunks[3], app);

    if let Some(permission) = &app.permission {
        draw_permission_modal(frame, frame.area(), permission);
    }
    if let Some(picker) = &app.picker {
        draw_model_picker(frame, frame.area(), picker, &app.session_model);
    }
    if let Some(picker) = &app.slash_picker {
        draw_slash_picker(frame, frame.area(), picker);
    }
}

fn draw_header(frame: &mut Frame<'_>, area: Rect, app: &TuiApp) {
    let title = Line::from(vec![
        Span::styled(
            "s",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "a",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "i",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "c",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "o",
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "d",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "e",
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  rust frontend + rust backend",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    let status = if app.busy {
        format!("{} running", app.spinner())
    } else if app.ready {
        "connected".to_string()
    } else {
        "connecting".to_string()
    };
    let info = vec![
        title,
        Line::from(vec![
            kv(
                "model",
                &empty_fallback(&app.session_model, "unset"),
                Color::Green,
            ),
            Span::raw("  "),
            kv(
                "profile",
                &empty_fallback(&app.session_profile, "unset"),
                Color::Blue,
            ),
            Span::raw("  "),
            kv(
                "permission",
                &empty_fallback(&app.session_permission, "unset"),
                Color::Yellow,
            ),
        ]),
        Line::from(vec![
            kv(
                "workspace",
                &empty_fallback(&app.session_workspace, &app.repo_root),
                Color::Cyan,
            ),
            Span::raw("  "),
            kv(
                "backend",
                &status,
                if app.ready {
                    Color::Green
                } else {
                    Color::Yellow
                },
            ),
        ]),
    ];
    let paragraph = Paragraph::new(info).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(paragraph, area);
}

fn draw_messages(frame: &mut Frame<'_>, area: Rect, app: &mut TuiApp, lines: Vec<Line<'static>>) {
    let max_scroll = transcript_max_scroll(&lines, area);
    app.last_transcript_max_scroll = max_scroll;
    if app.follow_latest {
        app.scroll = max_scroll;
    } else {
        app.scroll = app.scroll.min(max_scroll);
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Transcript"))
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));
    frame.render_widget(paragraph, area);
}

fn build_transcript_lines(app: &TuiApp) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if app.messages.is_empty() && app.draft_assistant.is_empty() {
        lines.push(Line::from(Span::styled(
            "等待输入。这里会显示流式输出、工具事件、权限请求和系统结果。",
            Style::default().fg(Color::DarkGray),
        )));
    }

    for message in &app.messages {
        lines.extend(message_to_lines(message));
    }
    if !app.draft_assistant.is_empty() {
        lines.extend(message_to_lines(&MessageItem {
            kind: MessageKind::Assistant,
            title: "Saicode".to_string(),
            body: app.draft_assistant.clone(),
            meta: Some(format!("{} streaming", app.spinner())),
        }));
    }
    lines
}

fn draw_input(frame: &mut Frame<'_>, area: Rect, app: &TuiApp) {
    let cursor = clamp_to_char_boundary(&app.input, app.cursor);
    let mut input_spans = vec![Span::styled(
        "› ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )];
    if app.input.is_empty() {
        input_spans.push(Span::styled(
            "输入消息或 /命令；输入 /model 打开模型列表",
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        input_spans.push(Span::raw(app.input.clone()));
    }
    let paragraph = Paragraph::new(Line::from(input_spans))
        .block(Block::default().borders(Borders::ALL).title("Prompt"));
    frame.render_widget(paragraph, area);
    if should_show_input_cursor(app) {
        frame.set_cursor_position(input_cursor_position(area, &app.input, cursor));
    }
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, app: &TuiApp) {
    frame.render_widget(Paragraph::new(footer_lines(app)), area);
}

fn should_show_input_cursor(app: &TuiApp) -> bool {
    app.permission.is_none() && app.picker.is_none()
}

fn input_cursor_position(area: Rect, input: &str, cursor: usize) -> Position {
    let clamped = clamp_to_char_boundary(input, cursor);
    let before = &input[..clamped];
    let before_width = Line::from(before.to_string()).width() as u16;
    Position::new(area.x + 1 + 2 + before_width, area.y + 1)
}

fn footer_lines(app: &TuiApp) -> Vec<Line<'static>> {
    let context_color = if app.context_percent >= app.auto_compact_percent as f64 {
        Color::Red
    } else if app.context_percent >= 60.0 {
        Color::Yellow
    } else {
        Color::Green
    };
    let mut lines = vec![
        Line::from(vec![
            kv(
                "model",
                &empty_fallback(&app.session_model, "unset"),
                Color::Green,
            ),
            Span::raw("  "),
            kv(
                "provider",
                &empty_fallback(&app.session_provider, "unset"),
                Color::Magenta,
            ),
            Span::raw("  "),
            kv(
                "wire",
                &empty_fallback(&app.session_wire_model, "unset"),
                Color::Blue,
            ),
            Span::raw("  "),
            kv(
                "usage",
                &format!("{}/{}", app.usage_input, app.usage_output),
                Color::Cyan,
            ),
        ]),
        Line::from(vec![
            kv(
                "context",
                &format!(
                    "{}/{} ({:.1}%)",
                    app.context_tokens, app.context_window_tokens, app.context_percent
                ),
                context_color,
            ),
            Span::raw("  "),
            kv(
                "parts",
                &format!(
                    "s:{} sys:{} t:{}",
                    app.context_session_tokens, app.context_system_tokens, app.context_tool_tokens
                ),
                Color::DarkGray,
            ),
            Span::raw("  "),
            kv(
                "compact",
                &format!(
                    "@{}% count={}",
                    app.auto_compact_percent, app.session_compaction_count
                ),
                Color::Yellow,
            ),
            Span::raw("  "),
            kv(
                "messages",
                &app.session_message_count.to_string(),
                Color::White,
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{} status", if app.busy { app.spinner() } else { "•" }),
                Style::default().fg(if app.busy {
                    Color::Yellow
                } else {
                    Color::Green
                }),
            ),
            Span::raw("  "),
            Span::raw(
                app.status_lines
                    .last()
                    .cloned()
                    .unwrap_or_else(|| "Idle.".to_string()),
            ),
        ]),
    ];
    if app.status_lines.len() > 1 {
        lines.push(Line::from(vec![
            Span::styled("prev", Style::default().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::raw(app.status_lines[app.status_lines.len() - 2].clone()),
        ]));
    }
    lines.push(Line::from(vec![
        kv(
            "profile",
            &empty_fallback(&app.session_profile, "unset"),
            Color::Blue,
        ),
        Span::raw("  "),
        kv(
            "permission",
            &empty_fallback(&app.session_permission, "unset"),
            Color::Yellow,
        ),
        Span::raw("  "),
        kv("mouse", app.mouse_mode.label(), Color::Cyan),
        Span::raw("  "),
        Span::styled(
            "↑↓ history/list  Tab accept /cmd  PgUp/PgDn scroll  F2 /model  F3 mouse  Ctrl+C exit",
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    lines
}

fn draw_model_picker(frame: &mut Frame<'_>, area: Rect, picker: &PickerState, current_model: &str) {
    let popup = centered_rect(72, 18, area);
    let mut lines = vec![Line::from(vec![
        Span::styled(
            "Model picker",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "  filter: {}",
                if picker.query.is_empty() {
                    "(none)"
                } else {
                    &picker.query
                }
            ),
            Style::default().fg(Color::DarkGray),
        ),
    ])];
    lines.push(Line::from(Span::styled(
        "↑↓ select  Enter confirm  Esc close  typing filters",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));
    if picker.filtered.is_empty() {
        lines.push(Line::from(Span::styled(
            "没有匹配项",
            Style::default().fg(Color::Red),
        )));
    } else {
        for (visible_index, candidate_index) in picker.filtered.iter().enumerate().take(10) {
            let candidate = &picker.items[*candidate_index];
            let prefix = if visible_index == picker.selected {
                "❯ "
            } else {
                "  "
            };
            let style = if visible_index == picker.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if candidate.id == current_model {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(candidate.label.clone(), style),
                Span::styled(format!("  {}", candidate.id), style.fg(Color::DarkGray)),
            ]));
        }
    }
    let block = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" /model "))
        .wrap(Wrap { trim: false });
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
}

fn draw_permission_modal(frame: &mut Frame<'_>, area: Rect, permission: &PermissionState) {
    let popup = centered_rect(70, 16, area);
    let allow_style = if permission.allow_selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let deny_style = if permission.allow_selected {
        Style::default().fg(Color::Red)
    } else {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD)
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("Tool: {}", permission.tool_name),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "current={}  required={}",
            permission.current_mode, permission.required_mode
        )),
    ];
    if let Some(reason) = &permission.reason {
        lines.push(Line::from(reason.clone()));
    }
    lines.push(Line::from(""));
    if let Some(input) = &permission.input {
        for line in clip_text(input, 6, 1200).lines() {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" allow ", allow_style),
        Span::raw("  "),
        Span::styled(" deny ", deny_style),
        Span::raw("  "),
        Span::styled("y/n or Enter", Style::default().fg(Color::DarkGray)),
    ]));
    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Permission "));
    frame.render_widget(Clear, popup);
    frame.render_widget(paragraph, popup);
}

fn draw_slash_picker(frame: &mut Frame<'_>, area: Rect, picker: &SlashPickerState) {
    let popup = centered_rect(86, 20, area);
    let mut lines = vec![Line::from(vec![
        Span::styled(
            "Slash commands",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "  filter: {}",
                if picker.query.is_empty() {
                    "(all)"
                } else {
                    &picker.query
                }
            ),
            Style::default().fg(Color::DarkGray),
        ),
    ])];
    lines.push(Line::from(Span::styled(
        "↑↓ select  Tab/Enter accept  Enter submits exact command  Esc close",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));
    if picker.filtered.is_empty() {
        lines.push(Line::from(Span::styled(
            "没有匹配的斜杠命令",
            Style::default().fg(Color::Red),
        )));
    } else {
        for (visible_index, candidate_index) in picker.filtered.iter().enumerate().take(12) {
            let candidate = &picker.items[*candidate_index];
            let selected = visible_index == picker.selected;
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(vec![
                Span::styled(if selected { "❯ " } else { "  " }, style),
                Span::styled(format!("{:<18}", candidate.label), style),
                Span::styled(candidate.summary.clone(), style.fg(Color::DarkGray)),
            ]));
        }
    }
    let block = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" / "))
        .wrap(Wrap { trim: false });
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
}

fn message_to_lines(message: &MessageItem) -> Vec<Line<'static>> {
    let (color, icon) = match message.kind {
        MessageKind::User => (Color::Cyan, "›"),
        MessageKind::Assistant => (Color::Green, "◆"),
        MessageKind::Tool => (Color::Yellow, "◇"),
        MessageKind::System => (Color::Blue, "●"),
        MessageKind::Error => (Color::Red, "✖"),
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!("{icon} {}", message.title),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            message
                .meta
                .as_ref()
                .map(|meta| format!(" · {meta}"))
                .unwrap_or_default(),
            Style::default().fg(Color::DarkGray),
        ),
    ])];
    for line in display_body_for_message(message) {
        lines.push(Line::from(Span::raw(line.to_string())));
    }
    lines.push(Line::from(""));
    lines
}

fn display_body_for_message(message: &MessageItem) -> Vec<String> {
    match message.kind {
        MessageKind::Assistant => compact_assistant_display(&message.body),
        _ => message.body.lines().map(str::to_string).collect(),
    }
}

fn compact_assistant_display(body: &str) -> Vec<String> {
    let mut output = Vec::new();
    let normalized = body.replace("\r\n", "\n");
    let mut hidden_code_lines = 0usize;
    let mut hidden_structured_lines = 0usize;
    let mut in_fenced_block = false;

    for line in normalized.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_fenced_block {
                in_fenced_block = false;
                push_hidden_block_summary(&mut output, &mut hidden_code_lines, "代码");
            } else {
                in_fenced_block = true;
            }
            continue;
        }

        if in_fenced_block {
            hidden_code_lines += 1;
            continue;
        }

        if assistant_line_should_hide(trimmed) {
            hidden_structured_lines += 1;
            continue;
        }

        push_hidden_block_summary(
            &mut output,
            &mut hidden_structured_lines,
            "代码或结构化内容",
        );
        output.push(line.to_string());
    }

    push_hidden_block_summary(&mut output, &mut hidden_code_lines, "代码");
    push_hidden_block_summary(
        &mut output,
        &mut hidden_structured_lines,
        "代码或结构化内容",
    );

    if output.is_empty() {
        output.push("（本段内容已折叠为代码或结构化输出）".to_string());
    }

    output
}

fn push_hidden_block_summary(output: &mut Vec<String>, hidden_lines: &mut usize, label: &str) {
    if *hidden_lines == 0 {
        return;
    }
    output.push(format!("（已折叠 {} 行{}）", *hidden_lines, label));
    *hidden_lines = 0;
}

fn assistant_line_should_hide(trimmed: &str) -> bool {
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with("fn ")
        || trimmed.starts_with("pub ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("class ")
        || trimmed.starts_with("interface ")
        || trimmed.starts_with("const ")
        || trimmed.starts_with("let ")
        || trimmed.starts_with("var ")
        || trimmed.starts_with("if ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("return ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("export ")
        || trimmed.starts_with('#')
    {
        return true;
    }

    if trimmed.contains("->")
        || trimmed.contains("=>")
        || trimmed.contains("::")
        || trimmed.contains("</")
        || trimmed.contains("/>")
        || trimmed.contains('{')
        || trimmed.contains('}')
        || trimmed.contains(';')
    {
        return true;
    }

    if trimmed.chars().any(is_box_drawing) {
        return true;
    }

    let total = trimmed.chars().count();
    if total < 12 {
        return false;
    }
    let cjk = trimmed.chars().filter(|ch| is_cjk(*ch)).count();
    let ascii_word = trimmed
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '/' | '\\' | '-' | '.' | '|'))
        .count();
    let structural = trimmed
        .chars()
        .filter(|ch| {
            matches!(
                ch,
                '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '=' | '+' | '*' | ':' | ';' | '`'
            ) || is_box_drawing(*ch)
        })
        .count();

    cjk == 0 && (ascii_word + structural) * 10 / total >= 6
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x4E00..=0x9FFF
            | 0x3400..=0x4DBF
            | 0x3040..=0x309F
            | 0x30A0..=0x30FF
            | 0xAC00..=0xD7AF
    )
}

fn is_box_drawing(ch: char) -> bool {
    matches!(ch as u32, 0x2500..=0x257F)
}

fn build_slash_command_candidates(cwd: &Path) -> Vec<SlashCommandCandidate> {
    let snapshot = build_command_registry_snapshot_with_cwd(
        &CommandRegistryContext::for_surface(CommandSurface::Bridge, true),
        &[],
        cwd,
    );
    let mut items = snapshot
        .session_commands
        .iter()
        .map(|descriptor| {
            let insert_text = preferred_slash_insert_text(&descriptor.name, &descriptor.aliases);
            SlashCommandCandidate {
                label: insert_text.clone(),
                insert_text,
                summary: descriptor.description.clone(),
                aliases: descriptor.aliases.clone(),
                rank: slash_command_rank(&descriptor.name, &descriptor.aliases),
            }
        })
        .collect::<Vec<_>>();
    items.sort_by_key(|item| (item.rank, item.label.clone()));
    items
}

fn preferred_slash_insert_text(name: &str, aliases: &[String]) -> String {
    if name == "clear" && aliases.iter().any(|alias| alias == "new") {
        "/new".to_string()
    } else {
        format!("/{name}")
    }
}

fn slash_command_rank(name: &str, aliases: &[String]) -> usize {
    let key = if name == "clear" && aliases.iter().any(|alias| alias == "new") {
        "new"
    } else {
        name
    };
    COMMON_SLASH_COMMANDS
        .iter()
        .position(|candidate| *candidate == key)
        .unwrap_or(COMMON_SLASH_COMMANDS.len() + 100)
}

fn transcript_max_scroll(lines: &[Line<'static>], area: Rect) -> u16 {
    let inner_width = area.width.saturating_sub(2).max(1);
    let inner_height = area.height.saturating_sub(2);
    let rendered_lines = lines
        .iter()
        .map(|line| {
            let width = line.width();
            if width == 0 {
                1
            } else {
                width.div_ceil(inner_width as usize)
            }
        })
        .sum::<usize>();
    rendered_lines.saturating_sub(inner_height as usize) as u16
}

fn kv(name: &str, value: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!("{name}={value}"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width.saturating_sub(2)).max(10);
    let height = height.min(area.height.saturating_sub(2)).max(6);
    Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    }
}

fn pretty_json(value: Option<&Value>) -> String {
    value
        .and_then(|value| serde_json::to_string_pretty(value).ok())
        .unwrap_or_default()
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn clip_text(text: &str, max_lines: usize, max_chars: usize) -> String {
    let normalized = text.replace("\r\n", "\n");
    let truncated = if normalized.len() > max_chars {
        format!(
            "{}\n…({} chars omitted)",
            &normalized[..max_chars],
            normalized.len() - max_chars
        )
    } else {
        normalized
    };
    let lines = truncated.lines().collect::<Vec<_>>();
    if lines.len() <= max_lines {
        return truncated;
    }
    format!(
        "{}\n…({} lines omitted)",
        lines[..max_lines].join("\n"),
        lines.len() - max_lines
    )
}

fn clamp_to_char_boundary(input: &str, cursor: usize) -> usize {
    if cursor >= input.len() {
        return input.len();
    }
    if input.is_char_boundary(cursor) {
        cursor
    } else {
        previous_char_boundary(input, cursor)
    }
}

fn previous_char_boundary(input: &str, cursor: usize) -> usize {
    let mut cursor = cursor.min(input.len());
    if cursor == 0 {
        return 0;
    }
    while cursor > 0 && !input.is_char_boundary(cursor) {
        cursor -= 1;
    }
    if cursor == 0 {
        0
    } else {
        input[..cursor]
            .char_indices()
            .last()
            .map(|(index, _)| index)
            .unwrap_or(0)
    }
}

fn next_char_boundary(input: &str, cursor: usize) -> usize {
    let cursor = clamp_to_char_boundary(input, cursor);
    if cursor >= input.len() {
        return input.len();
    }
    input[cursor..]
        .chars()
        .next()
        .map(|ch| cursor + ch.len_utf8())
        .unwrap_or(input.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_message_keeps_full_body() {
        let cwd = Path::new("/tmp");
        let mut app = TuiApp::new(cwd);
        let body = (0..20)
            .map(|index| format!("line-{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        app.add_message(MessageKind::Assistant, "Assistant", body.clone(), None);

        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].body, body);
        assert!(!app.messages[0].body.contains("omitted"));
    }

    #[test]
    fn assistant_display_hides_fenced_code_blocks() {
        let lines = compact_assistant_display(
            "先检查问题\n```rust\nfn main() {\n    println!(\"hi\");\n}\n```\n再继续修复",
        );

        assert_eq!(
            lines,
            vec![
                "先检查问题".to_string(),
                "（已折叠 3 行代码）".to_string(),
                "再继续修复".to_string()
            ]
        );
    }

    #[test]
    fn assistant_display_hides_structured_ascii_blocks() {
        let lines = compact_assistant_display("处理流程如下\n┌────┐\n│abc│\n└────┘\n最后输出结果");

        assert_eq!(
            lines,
            vec![
                "处理流程如下".to_string(),
                "（已折叠 3 行代码或结构化内容）".to_string(),
                "最后输出结果".to_string()
            ]
        );
    }

    #[test]
    fn input_cursor_position_counts_wide_characters() {
        let area = Rect {
            x: 10,
            y: 20,
            width: 30,
            height: 3,
        };

        let position = input_cursor_position(area, "你a", '你'.len_utf8());

        assert_eq!(position.x, 15);
        assert_eq!(position.y, 21);
    }

    #[test]
    fn transcript_scroll_counts_wrapped_lines() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 5,
        };
        let lines = vec![Line::from(
            "this is a long transcript line that must wrap several times",
        )];

        let max_scroll = transcript_max_scroll(&lines, area);

        assert!(max_scroll > 0);
    }

    #[test]
    fn utf8_cursor_navigation_uses_char_boundaries() {
        let cwd = Path::new("/tmp");
        let mut app = TuiApp::new(cwd);
        app.insert_char('、');

        assert_eq!(app.cursor, '、'.len_utf8());
        assert_eq!(previous_char_boundary(&app.input, app.cursor), 0);
        assert_eq!(next_char_boundary(&app.input, 0), '、'.len_utf8());
    }

    #[test]
    fn backspace_and_delete_remove_full_utf8_characters() {
        let cwd = Path::new("/tmp");
        let mut app = TuiApp::new(cwd);
        app.input = "a、b".to_string();
        app.cursor = "a".len();

        app.delete_char();
        assert_eq!(app.input, "ab");
        assert_eq!(app.cursor, "a".len());

        app.cursor = app.input.len();
        app.backspace();
        assert_eq!(app.input, "a");
        assert_eq!(app.cursor, "a".len());
    }

    #[test]
    fn clamp_to_char_boundary_handles_mid_utf8_cursor() {
        assert_eq!(clamp_to_char_boundary("、", 1), 0);
        assert_eq!(clamp_to_char_boundary("中a", 2), 0);
        assert_eq!(clamp_to_char_boundary("中a", 3), 3);
    }

    #[test]
    fn scroll_up_leaves_follow_latest_and_moves_toward_history() {
        let cwd = Path::new("/tmp");
        let mut app = TuiApp::new(cwd);
        app.follow_latest = true;
        app.last_transcript_max_scroll = 20;

        app.scroll_up(3);

        assert_eq!(app.scroll, 17);
        assert!(!app.follow_latest);
    }

    #[test]
    fn scroll_down_returns_to_follow_latest_when_reaching_bottom() {
        let cwd = Path::new("/tmp");
        let mut app = TuiApp::new(cwd);
        app.follow_latest = false;
        app.scroll = 17;
        app.last_transcript_max_scroll = 20;

        app.scroll_down(3);

        assert_eq!(app.scroll, 20);
        assert!(app.follow_latest);
    }
}

fn empty_fallback<'a>(value: &'a str, fallback: &'a str) -> String {
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}
