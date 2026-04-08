use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::types::{
    AgentInput, AskUserQuestionInput, BriefInput, ConfigInput, CronCreateInput, CronDeleteInput,
    EditFileInput, EnterPlanModeInput, ExitPlanModeInput, GlobSearchInputValue, LspInput,
    McpAuthInput, McpResourceInput, McpToolInput, NotebookEditInput, PowerShellInput,
    ReadFileInput, RemoteTriggerInput, ReplInput, SleepInput, StructuredOutputInput,
    TaskCreateInput, TaskIdInput, TaskUpdateInput, TeamCreateInput, TeamDeleteInput,
    TestingPermissionInput, TodoWriteInput, ToolSearchInput, WebBrowserInput, WebFetchInput,
    WebSearchInput, WriteFileInput,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskRecord {
    task_id: String,
    prompt: String,
    description: Option<String>,
    status: String,
    created_at_ms: u64,
    updated_at_ms: u64,
    output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TeamRecord {
    team_id: String,
    name: String,
    task_count: usize,
    created_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CronRecord {
    cron_id: String,
    schedule: String,
    prompt: String,
    description: Option<String>,
    created_at_ms: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct DispatchState {
    tasks: BTreeMap<String, TaskRecord>,
    teams: BTreeMap<String, TeamRecord>,
    crons: BTreeMap<String, CronRecord>,
}

fn dispatch_state() -> &'static Mutex<DispatchState> {
    static STATE: OnceLock<Mutex<DispatchState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(load_dispatch_state()))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn next_id(prefix: &str) -> String {
    format!("{prefix}_{:x}", now_millis())
}

fn dispatch_state_path() -> PathBuf {
    if let Ok(path) = std::env::var("SAICODE_DISPATCH_STATE_PATH") {
        return PathBuf::from(path);
    }
    if let Ok(path) = std::env::var("SAICODE_CONFIG_HOME") {
        return PathBuf::from(path).join("runtime-dispatch-state.json");
    }
    if let Ok(path) = std::env::var("CLAW_CONFIG_HOME") {
        return PathBuf::from(path).join("runtime-dispatch-state.json");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".saicode")
            .join("runtime-dispatch-state.json");
    }
    PathBuf::from(".saicode").join("runtime-dispatch-state.json")
}

fn load_dispatch_state() -> DispatchState {
    let path = dispatch_state_path();
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => DispatchState::default(),
    }
}

fn save_dispatch_state(state: &DispatchState) -> Result<(), String> {
    let path = dispatch_state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let payload = serde_json::to_string_pretty(state).map_err(|error| error.to_string())?;
    fs::write(path, payload).map_err(|error| error.to_string())
}

pub fn execute_tool(name: &str, input: &Value) -> Result<String, String> {
    match name {
        "bash" => from_value::<runtime::BashCommandInput>(input).and_then(run_bash),
        "read_file" => from_value::<ReadFileInput>(input).and_then(run_read_file),
        "write_file" => from_value::<WriteFileInput>(input).and_then(run_write_file),
        "edit_file" => from_value::<EditFileInput>(input).and_then(run_edit_file),
        "glob_search" => from_value::<GlobSearchInputValue>(input).and_then(run_glob_search),
        "grep_search" => from_value::<runtime::GrepSearchInput>(input).and_then(run_grep_search),
        "WebFetch" => from_value::<WebFetchInput>(input).and_then(run_web_fetch),
        "WebSearch" => from_value::<WebSearchInput>(input).and_then(run_web_search),
        "TodoWrite" => from_value::<TodoWriteInput>(input).and_then(run_todo_write),
        "Skill" => from_value::<crate::types::SkillInput>(input).and_then(run_skill),
        "Agent" => from_value::<AgentInput>(input).and_then(run_agent),
        "ToolSearch" => from_value::<ToolSearchInput>(input).and_then(run_tool_search),
        "NotebookEdit" => from_value::<NotebookEditInput>(input).and_then(run_notebook_edit),
        "Sleep" => from_value::<SleepInput>(input).and_then(run_sleep),
        "SendUserMessage" | "Brief" => from_value::<BriefInput>(input).and_then(run_brief),
        "Config" => from_value::<ConfigInput>(input).and_then(run_config),
        "EnterPlanMode" => from_value::<EnterPlanModeInput>(input).and_then(run_enter_plan_mode),
        "ExitPlanMode" => from_value::<ExitPlanModeInput>(input).and_then(run_exit_plan_mode),
        "StructuredOutput" => {
            from_value::<StructuredOutputInput>(input).and_then(run_structured_output)
        }
        "REPL" => from_value::<ReplInput>(input).and_then(run_repl),
        "PowerShell" => from_value::<PowerShellInput>(input).and_then(run_powershell),
        "AskUserQuestion" => {
            from_value::<AskUserQuestionInput>(input).and_then(run_ask_user_question)
        }
        "TaskCreate" => from_value::<TaskCreateInput>(input).and_then(run_task_create),
        "TaskGet" => from_value::<TaskIdInput>(input).and_then(run_task_get),
        "TaskList" => run_task_list(input.clone()),
        "TaskStop" => from_value::<TaskIdInput>(input).and_then(run_task_stop),
        "TaskUpdate" => from_value::<TaskUpdateInput>(input).and_then(run_task_update),
        "TaskOutput" => from_value::<TaskIdInput>(input).and_then(run_task_output),
        "TeamCreate" => from_value::<TeamCreateInput>(input).and_then(run_team_create),
        "TeamDelete" => from_value::<TeamDeleteInput>(input).and_then(run_team_delete),
        "CronCreate" => from_value::<CronCreateInput>(input).and_then(run_cron_create),
        "CronDelete" => from_value::<CronDeleteInput>(input).and_then(run_cron_delete),
        "CronList" => run_cron_list(input.clone()),
        "LSP" => from_value::<LspInput>(input).and_then(run_lsp),
        "ListMcpResources" => {
            from_value::<McpResourceInput>(input).and_then(run_list_mcp_resources)
        }
        "ReadMcpResource" => from_value::<McpResourceInput>(input).and_then(run_read_mcp_resource),
        "McpAuth" => from_value::<McpAuthInput>(input).and_then(run_mcp_auth),
        "RemoteTrigger" => from_value::<RemoteTriggerInput>(input).and_then(run_remote_trigger),
        "MCP" => from_value::<McpToolInput>(input).and_then(run_mcp_tool),
        "TestingPermission" => {
            from_value::<TestingPermissionInput>(input).and_then(run_testing_permission)
        }
        "WebBrowser" => from_value::<WebBrowserInput>(input).and_then(run_web_browser),
        _ => Err(format!("unsupported tool: {name}")),
    }
}

fn run_ask_user_question(input: AskUserQuestionInput) -> Result<String, String> {
    let mut result = json!({
        "question": input.question,
        "status": "pending",
        "message": "Waiting for user response"
    });
    if let Some(options) = &input.options {
        result["options"] = json!(options);
    }
    to_pretty_json(result)
}

fn run_task_create(input: TaskCreateInput) -> Result<String, String> {
    let task_id = next_id("task");
    let record = TaskRecord {
        task_id: task_id.clone(),
        prompt: input.prompt,
        description: input.description,
        status: "created".to_string(),
        created_at_ms: now_millis(),
        updated_at_ms: now_millis(),
        output: String::new(),
    };
    let mut state = dispatch_state()
        .lock()
        .map_err(|_| "failed to lock task registry".to_string())?;
    state.tasks.insert(task_id.clone(), record.clone());
    save_dispatch_state(&state)?;
    to_pretty_json(record)
}

fn run_task_get(input: TaskIdInput) -> Result<String, String> {
    let state = dispatch_state()
        .lock()
        .map_err(|_| "failed to lock task registry".to_string())?;
    let record = state
        .tasks
        .get(&input.task_id)
        .ok_or_else(|| format!("unknown task: {}", input.task_id))?;
    to_pretty_json(record)
}

fn run_task_list(_input: Value) -> Result<String, String> {
    let state = dispatch_state()
        .lock()
        .map_err(|_| "failed to lock task registry".to_string())?;
    let tasks = state.tasks.values().cloned().collect::<Vec<_>>();
    to_pretty_json(json!({ "tasks": tasks, "count": tasks.len() }))
}

fn run_task_stop(input: TaskIdInput) -> Result<String, String> {
    let mut state = dispatch_state()
        .lock()
        .map_err(|_| "failed to lock task registry".to_string())?;
    let record = state
        .tasks
        .get_mut(&input.task_id)
        .ok_or_else(|| format!("unknown task: {}", input.task_id))?;
    record.status = "stopped".to_string();
    record.updated_at_ms = now_millis();
    let response = record.clone();
    save_dispatch_state(&state)?;
    to_pretty_json(response)
}

fn run_task_update(input: TaskUpdateInput) -> Result<String, String> {
    let mut state = dispatch_state()
        .lock()
        .map_err(|_| "failed to lock task registry".to_string())?;
    let record = state
        .tasks
        .get_mut(&input.task_id)
        .ok_or_else(|| format!("unknown task: {}", input.task_id))?;
    if !record.output.is_empty() {
        record.output.push('\n');
    }
    record.output.push_str(&input.message);
    record.status = "updated".to_string();
    record.updated_at_ms = now_millis();
    let response = record.clone();
    save_dispatch_state(&state)?;
    to_pretty_json(response)
}

fn run_task_output(input: TaskIdInput) -> Result<String, String> {
    let state = dispatch_state()
        .lock()
        .map_err(|_| "failed to lock task registry".to_string())?;
    let record = state
        .tasks
        .get(&input.task_id)
        .ok_or_else(|| format!("unknown task: {}", input.task_id))?;
    to_pretty_json(json!({
        "task_id": record.task_id,
        "status": record.status,
        "output": record.output,
    }))
}

fn run_team_create(input: TeamCreateInput) -> Result<String, String> {
    let record = TeamRecord {
        team_id: next_id("team"),
        name: input.name,
        task_count: input.tasks.len(),
        created_at_ms: now_millis(),
    };
    let mut state = dispatch_state()
        .lock()
        .map_err(|_| "failed to lock team registry".to_string())?;
    state.teams.insert(record.team_id.clone(), record.clone());
    save_dispatch_state(&state)?;
    to_pretty_json(record)
}

fn run_team_delete(input: TeamDeleteInput) -> Result<String, String> {
    let mut state = dispatch_state()
        .lock()
        .map_err(|_| "failed to lock team registry".to_string())?;
    let existed = state.teams.remove(&input.team_id).is_some();
    save_dispatch_state(&state)?;
    to_pretty_json(json!({
        "team_id": input.team_id,
        "status": if existed { "deleted" } else { "missing" },
    }))
}

fn run_cron_create(input: CronCreateInput) -> Result<String, String> {
    let record = CronRecord {
        cron_id: next_id("cron"),
        schedule: input.schedule,
        prompt: input.prompt,
        description: input.description,
        created_at_ms: now_millis(),
    };
    let mut state = dispatch_state()
        .lock()
        .map_err(|_| "failed to lock cron registry".to_string())?;
    state.crons.insert(record.cron_id.clone(), record.clone());
    save_dispatch_state(&state)?;
    to_pretty_json(record)
}

fn run_cron_delete(input: CronDeleteInput) -> Result<String, String> {
    let mut state = dispatch_state()
        .lock()
        .map_err(|_| "failed to lock cron registry".to_string())?;
    let existed = state.crons.remove(&input.cron_id).is_some();
    save_dispatch_state(&state)?;
    to_pretty_json(json!({
        "cron_id": input.cron_id,
        "status": if existed { "deleted" } else { "missing" },
    }))
}

fn run_cron_list(_input: Value) -> Result<String, String> {
    let state = dispatch_state()
        .lock()
        .map_err(|_| "failed to lock cron registry".to_string())?;
    let crons = state.crons.values().cloned().collect::<Vec<_>>();
    to_pretty_json(json!({ "crons": crons, "count": crons.len() }))
}

fn run_lsp(input: LspInput) -> Result<String, String> {
    let operation = match input.action.as_str() {
        "symbols" => {
            if input.path.is_some() {
                "document_symbols"
            } else {
                "workspace_symbols"
            }
        }
        "definition" => "definition",
        "references" => "references",
        "hover" => "hover",
        "document_symbols" => "document_symbols",
        "workspace_symbols" => "workspace_symbols",
        other => return Err(format!("unsupported LSP action: {other}")),
    };
    let request = runtime::LspRequest {
        operation: operation.to_string(),
        file_path: input.path.clone().map(PathBuf::from),
        line: input.line,
        character: input.character,
        query: input.query.clone().or_else(|| {
            (operation == "workspace_symbols")
                .then(|| identifier_from_input(&input))
                .flatten()
        }),
    };
    let rt = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    let result = rt.block_on(runtime::execute_lsp_request(request))?;
    to_pretty_json(result)
}

fn run_list_mcp_resources(input: McpResourceInput) -> Result<String, String> {
    let server_name = input
        .server
        .ok_or_else(|| "server is required for ListMcpResources".to_string())?;
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let runtime_config = runtime::ConfigLoader::default_for(&cwd)
        .load()
        .map_err(|error| error.to_string())?;
    let mut manager = runtime::McpServerManager::from_runtime_config(&runtime_config);
    let rt = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    let resources = rt
        .block_on(manager.list_resources(&server_name))
        .map_err(|error| error.to_string())?;
    let unsupported = manager
        .unsupported_servers()
        .iter()
        .map(|server| {
            json!({
                "server_name": server.server_name,
                "transport": format!("{:?}", server.transport),
                "reason": server.reason,
            })
        })
        .collect::<Vec<_>>();
    to_pretty_json(json!({
        "server": server_name,
        "resources": resources,
        "unsupported_servers": unsupported,
    }))
}

fn run_read_mcp_resource(input: McpResourceInput) -> Result<String, String> {
    let server_name = input
        .server
        .ok_or_else(|| "server is required for ReadMcpResource".to_string())?;
    let uri = input
        .uri
        .ok_or_else(|| "uri is required for ReadMcpResource".to_string())?;
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let runtime_config = runtime::ConfigLoader::default_for(&cwd)
        .load()
        .map_err(|error| error.to_string())?;
    let mut manager = runtime::McpServerManager::from_runtime_config(&runtime_config);
    let rt = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    let result = rt
        .block_on(manager.read_resource(&server_name, &uri))
        .map_err(|error| error.to_string())?;
    to_pretty_json(json!({
        "server": server_name,
        "uri": uri,
        "content": result,
    }))
}

fn run_mcp_auth(input: McpAuthInput) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let runtime_config = runtime::ConfigLoader::default_for(&cwd)
        .load()
        .map_err(|error| error.to_string())?;
    let server = runtime_config
        .mcp()
        .get(&input.server)
        .ok_or_else(|| format!("unknown MCP server: {}", input.server))?;
    let transport = runtime::McpClientTransport::from_config(&server.config);
    let auth_required = matches!(
        &transport,
        runtime::McpClientTransport::Sse(remote)
            | runtime::McpClientTransport::Http(remote)
            | runtime::McpClientTransport::WebSocket(remote)
                if remote.auth.requires_user_auth()
    );
    to_pretty_json(json!({
        "server": input.server,
        "status": if auth_required { "auth_required" } else { "ready" },
        "transport": format!("{transport:?}"),
    }))
}

fn run_remote_trigger(input: RemoteTriggerInput) -> Result<String, String> {
    let method = input
        .method
        .unwrap_or_else(|| "GET".to_string())
        .to_ascii_uppercase();
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|error| error.to_string())?;
    let mut request = match method.as_str() {
        "GET" => client.get(&input.url),
        "POST" => client.post(&input.url),
        "PUT" => client.put(&input.url),
        "DELETE" => client.delete(&input.url),
        other => return Err(format!("unsupported RemoteTrigger method: {other}")),
    };
    if let Some(headers) = input.headers {
        if let Some(object) = headers.as_object() {
            for (name, value) in object {
                if let Some(value) = value.as_str() {
                    request = request.header(name, value);
                }
            }
        }
    }
    if let Some(body) = input.body {
        request = request.body(body);
    }
    let response = request.send().map_err(|error| error.to_string())?;
    let status = response.status();
    let url = response.url().to_string();
    let body = response.text().map_err(|error| error.to_string())?;
    to_pretty_json(json!({
        "url": url,
        "method": method,
        "status": status.as_u16(),
        "ok": status.is_success(),
        "body_preview": truncate_text(&body, 1000),
    }))
}

fn run_mcp_tool(input: McpToolInput) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let runtime_config = runtime::ConfigLoader::default_for(&cwd)
        .load()
        .map_err(|error| error.to_string())?;
    let mut manager = runtime::McpServerManager::from_runtime_config(&runtime_config);
    let rt = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    let tools = rt
        .block_on(manager.discover_tools())
        .map_err(|error| error.to_string())?;
    let resolved = tools
        .iter()
        .find(|tool| tool.server_name == input.server && tool.raw_name == input.tool)
        .cloned()
        .ok_or_else(|| {
            format!(
                "unknown MCP tool `{}` on server `{}`",
                input.tool, input.server
            )
        })?;
    let qualified = resolved.qualified_name.clone();
    let arguments = normalize_mcp_arguments(input.arguments, resolved.tool.input_schema.as_ref());
    let result = rt
        .block_on(manager.call_tool(&qualified, arguments))
        .map_err(|error| error.to_string())?;
    to_pretty_json(json!({
        "server": input.server,
        "tool": input.tool,
        "qualified_tool": qualified,
        "result": result,
    }))
}

fn normalize_mcp_arguments(arguments: Option<Value>, schema: Option<&Value>) -> Option<Value> {
    let Some(Value::Object(mut object)) = arguments else {
        return arguments;
    };

    let placeholder = object.remove("__compat_placeholder");
    if object.is_empty() {
        if let Some(placeholder_value) = placeholder {
            if let Some(field_name) = single_mcp_schema_field_name(schema) {
                object.insert(field_name, placeholder_value);
            } else {
                object.insert("__compat_placeholder".to_string(), placeholder_value);
            }
        }
    } else if let Some(placeholder_value) = placeholder {
        object.insert("__compat_placeholder".to_string(), placeholder_value);
    }

    Some(Value::Object(object))
}

fn single_mcp_schema_field_name(schema: Option<&Value>) -> Option<String> {
    let properties = schema
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object)?;
    let field_names = properties
        .keys()
        .filter(|name| name.as_str() != "__compat_placeholder")
        .cloned()
        .collect::<Vec<_>>();
    if field_names.len() == 1 {
        field_names.into_iter().next()
    } else {
        None
    }
}

fn run_testing_permission(input: TestingPermissionInput) -> Result<String, String> {
    to_pretty_json(json!({
        "action": input.action,
        "permitted": true,
        "message": "Testing permission tool stub"
    }))
}

fn run_web_browser(input: WebBrowserInput) -> Result<String, String> {
    let message = match input.action.as_str() {
        "navigate" => {
            let url = input.url.as_deref().ok_or("navigate requires a url")?;
            return to_pretty_json(crate::types::WebBrowserOutput {
                action: "navigate".into(),
                status: "info".into(),
                url: Some(url.into()),
                page_title: None,
                message: Some("WebBrowser tool: navigate action requires an external browser executor. Set WEB_BROWSER_EXECUTOR to enable.".into()),
                dom_length: None,
                screenshot: None,
            });
        }
        "screenshot" => "WebBrowser tool: screenshot action requires an external browser executor. Set WEB_BROWSER_EXECUTOR to enable.".to_string(),
        "extract_dom" => "WebBrowser tool: extract_dom action requires an external browser executor. Set WEB_BROWSER_EXECUTOR to enable.".to_string(),
        "click" | "type_text" | "scroll" => format!(
            "WebBrowser tool: {} action requires an external browser executor. Set WEB_BROWSER_EXECUTOR to enable.",
            input.action
        ),
        other => {
            return Err(format!(
                "WebBrowser: unknown action '{}'. Expected one of: navigate, screenshot, extract_dom, click, type_text, scroll",
                other
            ))
        }
    };

    to_pretty_json(crate::types::WebBrowserOutput {
        action: input.action,
        status: "info".into(),
        url: input.url,
        page_title: None,
        message: Some(message),
        dom_length: None,
        screenshot: None,
    })
}

pub(crate) fn from_value<T: for<'de> Deserialize<'de>>(input: &Value) -> Result<T, String> {
    serde_json::from_value(input.clone()).map_err(|error| error.to_string())
}

fn run_bash(input: runtime::BashCommandInput) -> Result<String, String> {
    let mut input = input;
    if input.namespace_restrictions.is_none() {
        input.namespace_restrictions = Some(false);
    }
    if input.isolate_network.is_none() {
        input.isolate_network = Some(false);
    }
    serde_json::to_string_pretty(&runtime::execute_bash(input).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

fn run_read_file(input: ReadFileInput) -> Result<String, String> {
    to_pretty_json(
        runtime::read_file(&input.path, input.offset, input.limit).map_err(io_to_string)?,
    )
}

fn run_write_file(input: WriteFileInput) -> Result<String, String> {
    to_pretty_json(runtime::write_file(&input.path, &input.content).map_err(io_to_string)?)
}

fn run_edit_file(input: EditFileInput) -> Result<String, String> {
    to_pretty_json(
        runtime::edit_file(
            &input.path,
            &input.old_string,
            &input.new_string,
            input.replace_all.unwrap_or(false),
        )
        .map_err(io_to_string)?,
    )
}

fn run_glob_search(input: GlobSearchInputValue) -> Result<String, String> {
    to_pretty_json(
        runtime::glob_search(&input.pattern, input.path.as_deref()).map_err(io_to_string)?,
    )
}

fn run_grep_search(input: runtime::GrepSearchInput) -> Result<String, String> {
    to_pretty_json(runtime::grep_search(&input).map_err(io_to_string)?)
}

fn run_web_fetch(input: WebFetchInput) -> Result<String, String> {
    saicode_frontline::local_tools::execute_stable_web_fetch(&input.url, &input.prompt)
}

fn run_web_search(input: WebSearchInput) -> Result<String, String> {
    let allowed = input.allowed_domains.unwrap_or_default();
    let blocked = input.blocked_domains.unwrap_or_default();
    saicode_frontline::local_tools::execute_stable_web_search(&input.query, &allowed, &blocked)
}

fn run_todo_write(input: TodoWriteInput) -> Result<String, String> {
    to_pretty_json(crate::todo_skill::execute_todo_write(input)?)
}

fn run_skill(input: crate::types::SkillInput) -> Result<String, String> {
    to_pretty_json(crate::todo_skill::execute_skill(input)?)
}

fn run_agent(input: AgentInput) -> Result<String, String> {
    to_pretty_json(crate::agent_spawn::execute_agent(input)?)
}

fn run_tool_search(input: ToolSearchInput) -> Result<String, String> {
    to_pretty_json(crate::agent_runtime::execute_tool_search(input))
}

fn run_notebook_edit(input: NotebookEditInput) -> Result<String, String> {
    to_pretty_json(crate::notebook::execute_notebook_edit(input)?)
}

fn run_sleep(input: SleepInput) -> Result<String, String> {
    to_pretty_json(crate::brief::execute_sleep(input)?)
}

fn run_brief(input: BriefInput) -> Result<String, String> {
    to_pretty_json(crate::brief::execute_brief(input)?)
}

fn run_config(input: ConfigInput) -> Result<String, String> {
    to_pretty_json(crate::config::execute_config(input)?)
}

fn run_enter_plan_mode(input: EnterPlanModeInput) -> Result<String, String> {
    to_pretty_json(crate::plan_mode::execute_enter_plan_mode(input)?)
}

fn run_exit_plan_mode(input: ExitPlanModeInput) -> Result<String, String> {
    to_pretty_json(crate::plan_mode::execute_exit_plan_mode(input)?)
}

fn run_structured_output(input: StructuredOutputInput) -> Result<String, String> {
    to_pretty_json(crate::repl::execute_structured_output(input)?)
}

fn run_repl(input: ReplInput) -> Result<String, String> {
    to_pretty_json(crate::repl::execute_repl(input)?)
}

fn run_powershell(input: PowerShellInput) -> Result<String, String> {
    to_pretty_json(crate::shell::execute_powershell(input).map_err(|error| error.to_string())?)
}

pub(crate) fn to_pretty_json<T: serde::Serialize>(value: T) -> Result<String, String> {
    serde_json::to_string_pretty(&value).map_err(|error| error.to_string())
}

pub(crate) fn io_to_string(error: std::io::Error) -> String {
    error.to_string()
}

fn identifier_from_input(input: &LspInput) -> Option<String> {
    let path = input.path.as_deref()?;
    let line_number = input.line?;
    let file = runtime::read_file(path, Some(line_number as usize), Some(1)).ok()?;
    let line = file.file.content.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    if let Some(character) = input.character {
        return identifier_at_character(line, character as usize);
    }
    identifier_at_character(line, line.len() / 2)
}

fn identifier_at_character(line: &str, character: usize) -> Option<String> {
    let chars = line.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return None;
    }
    let mut index = character.min(chars.len().saturating_sub(1));
    while index > 0 && !is_identifier_char(chars[index]) {
        index -= 1;
    }
    if !is_identifier_char(chars[index]) {
        return None;
    }
    let mut start = index;
    let mut end = index;
    while start > 0 && is_identifier_char(chars[start - 1]) {
        start -= 1;
    }
    while end + 1 < chars.len() && is_identifier_char(chars[end + 1]) {
        end += 1;
    }
    Some(chars[start..=end].iter().collect())
}

fn is_identifier_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    let mut truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        truncated.push_str("...");
    }
    truncated
}

#[cfg(test)]
mod tests {
    use super::normalize_mcp_arguments;
    use serde_json::json;

    #[test]
    fn mcp_placeholder_maps_to_single_schema_field() {
        let arguments = Some(json!({
            "__compat_placeholder": "acceptance"
        }));
        let schema = json!({
            "type": "object",
            "properties": {
                "text": { "type": "string" }
            },
            "required": ["text"]
        });

        let normalized = normalize_mcp_arguments(arguments, Some(&schema));

        assert_eq!(normalized, Some(json!({ "text": "acceptance" })));
    }

    #[test]
    fn mcp_placeholder_is_preserved_when_schema_is_ambiguous() {
        let arguments = Some(json!({
            "__compat_placeholder": "acceptance"
        }));
        let schema = json!({
            "type": "object",
            "properties": {
                "text": { "type": "string" },
                "mode": { "type": "string" }
            }
        });

        let normalized = normalize_mcp_arguments(arguments, Some(&schema));

        assert_eq!(
            normalized,
            Some(json!({ "__compat_placeholder": "acceptance" }))
        );
    }
}
