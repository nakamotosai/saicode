use api::{InputMessage, MessageRequest, ReasoningEffort, ToolChoice, ToolDefinition};
use runtime::{ContentBlock, ConversationMessage, MessageRole, Session, TokenUsage};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaicodeMessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SaicodeContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    ToolResult {
        tool_use_id: String,
        tool_name: String,
        output: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaicodeUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaicodeMessage {
    pub role: SaicodeMessageRole,
    pub blocks: Vec<SaicodeContentBlock>,
    pub usage: Option<SaicodeUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaicodeSessionSnapshot {
    pub session_id: String,
    pub message_count: usize,
    pub messages: Vec<SaicodeMessage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaicodeEffortLevel {
    Low,
    Medium,
    High,
    Max,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaicodeModelSelection {
    pub model: String,
    pub effort: Option<SaicodeEffortLevel>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaicodeRequestEnvelope {
    pub selection: SaicodeModelSelection,
    pub max_tokens: u32,
    pub messages: Vec<InputMessage>,
    pub system: Option<String>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub tool_choice: Option<ToolChoice>,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaicodePermissionMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
    Prompt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaicodeSessionStatus {
    Idle,
    RunningTurn,
    WaitingPermission,
    Interrupted,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaicodePermissionRequestEvent {
    pub request_id: String,
    pub tool_name: String,
    pub tool_use_id: String,
    pub description: String,
    pub input_json: Value,
    pub current_mode: SaicodePermissionMode,
    pub required_mode: SaicodePermissionMode,
    pub reason: Option<String>,
    pub suggested_ui: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaicodePermissionDecisionKind {
    Approve,
    Deny,
    Interrupt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaicodePermissionResolvedEvent {
    pub request_id: String,
    pub decision: SaicodePermissionDecisionKind,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaicodeSessionMeta {
    pub session_id: String,
    pub cwd: String,
    pub model: String,
    pub wire_model: String,
    pub permission_mode: SaicodePermissionMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SaicodeSessionStart {
    pub cwd: Option<String>,
    pub model: Option<String>,
    pub effort: Option<SaicodeEffortLevel>,
    pub permission_mode: Option<SaicodePermissionMode>,
    pub allowed_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub system_prompt: Option<String>,
    pub append_system_prompt: Option<String>,
    pub resume_path: Option<String>,
    pub no_session_persistence: bool,
    pub bare: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SaicodeSessionCommand {
    StartSession {
        payload: SaicodeSessionStart,
    },
    SubmitPrompt {
        prompt: String,
    },
    FetchSnapshot,
    ChangeModel {
        model: String,
    },
    ChangePermissionMode {
        mode: SaicodePermissionMode,
    },
    ApprovePermission {
        request_id: String,
        updated_input_json: Option<Value>,
        feedback: Option<String>,
    },
    DenyPermission {
        request_id: String,
        reason: Option<String>,
        feedback: Option<String>,
    },
    Interrupt,
    RunSlashCommand {
        command: String,
    },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SaicodeSessionEvent {
    SessionStarted {
        meta: SaicodeSessionMeta,
    },
    SessionSnapshot {
        snapshot: SaicodeSessionSnapshot,
    },
    StatusChanged {
        status: SaicodeSessionStatus,
    },
    AssistantTextDelta {
        text: String,
    },
    AssistantMessageDone {
        message: SaicodeMessage,
    },
    ToolResult {
        message: SaicodeMessage,
    },
    UsageUpdated {
        usage: SaicodeUsage,
    },
    PermissionRequest {
        request: SaicodePermissionRequestEvent,
    },
    PermissionResolved {
        resolution: SaicodePermissionResolvedEvent,
    },
    SlashCommandResult {
        output: String,
    },
    Error {
        message: String,
    },
    SessionEnded,
}

impl From<MessageRole> for SaicodeMessageRole {
    fn from(value: MessageRole) -> Self {
        match value {
            MessageRole::System => Self::System,
            MessageRole::User => Self::User,
            MessageRole::Assistant => Self::Assistant,
            MessageRole::Tool => Self::Tool,
        }
    }
}

impl From<&ContentBlock> for SaicodeContentBlock {
    fn from(value: &ContentBlock) -> Self {
        match value {
            ContentBlock::Text { text } => Self::Text { text: text.clone() },
            ContentBlock::ToolUse { id, name, input } => Self::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            },
            ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } => Self::ToolResult {
                tool_use_id: tool_use_id.clone(),
                tool_name: tool_name.clone(),
                output: output.clone(),
                is_error: *is_error,
            },
        }
    }
}

impl From<&TokenUsage> for SaicodeUsage {
    fn from(value: &TokenUsage) -> Self {
        Self {
            input_tokens: value.input_tokens,
            output_tokens: value.output_tokens,
            cache_creation_input_tokens: value.cache_creation_input_tokens,
            cache_read_input_tokens: value.cache_read_input_tokens,
        }
    }
}

impl From<&ConversationMessage> for SaicodeMessage {
    fn from(value: &ConversationMessage) -> Self {
        Self {
            role: value.role.into(),
            blocks: value.blocks.iter().map(SaicodeContentBlock::from).collect(),
            usage: value.usage.as_ref().map(SaicodeUsage::from),
        }
    }
}

impl From<SaicodeEffortLevel> for ReasoningEffort {
    fn from(value: SaicodeEffortLevel) -> Self {
        match value {
            SaicodeEffortLevel::Low => Self::Low,
            SaicodeEffortLevel::Medium => Self::Medium,
            SaicodeEffortLevel::High => Self::High,
            SaicodeEffortLevel::Max => Self::Max,
        }
    }
}

pub fn map_runtime_message(message: &ConversationMessage) -> SaicodeMessage {
    SaicodeMessage::from(message)
}

pub fn snapshot_session(session: &Session) -> SaicodeSessionSnapshot {
    SaicodeSessionSnapshot {
        session_id: session.session_id.clone(),
        message_count: session.messages.len(),
        messages: session.messages.iter().map(SaicodeMessage::from).collect(),
    }
}

pub fn build_message_request(envelope: SaicodeRequestEnvelope) -> MessageRequest {
    MessageRequest {
        model: envelope.selection.model,
        max_tokens: Some(envelope.max_tokens),
        messages: envelope.messages,
        system: envelope.system,
        tools: envelope.tools,
        tool_choice: envelope.tool_choice,
        reasoning_effort: envelope.selection.effort.map(ReasoningEffort::from),
        stream: envelope.stream,
    }
}

pub fn permission_mode_label(mode: SaicodePermissionMode) -> &'static str {
    match mode {
        SaicodePermissionMode::ReadOnly => "read-only",
        SaicodePermissionMode::WorkspaceWrite => "workspace-write",
        SaicodePermissionMode::DangerFullAccess => "danger-full-access",
        SaicodePermissionMode::Prompt => "prompt",
    }
}

pub fn map_runtime_permission_mode(mode: runtime::PermissionMode) -> SaicodePermissionMode {
    match mode {
        runtime::PermissionMode::ReadOnly => SaicodePermissionMode::ReadOnly,
        runtime::PermissionMode::WorkspaceWrite => SaicodePermissionMode::WorkspaceWrite,
        runtime::PermissionMode::DangerFullAccess | runtime::PermissionMode::Allow => {
            SaicodePermissionMode::DangerFullAccess
        }
        runtime::PermissionMode::Prompt => SaicodePermissionMode::Prompt,
    }
}

pub fn json_string(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        build_message_request, map_runtime_message, snapshot_session, SaicodeContentBlock,
        SaicodeEffortLevel, SaicodeMessageRole, SaicodeModelSelection, SaicodeRequestEnvelope,
    };
    use api::InputMessage;
    use runtime::{ContentBlock, ConversationMessage, MessageRole, Session, TokenUsage};

    #[test]
    fn maps_runtime_messages_into_saicode_surface_shape() {
        let message = ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![
                ContentBlock::Text {
                    text: "hello".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "tool-1".to_string(),
                    name: "Read".to_string(),
                    input: "{\"path\":\"/tmp/a\"}".to_string(),
                },
            ],
            usage: Some(TokenUsage {
                input_tokens: 12,
                output_tokens: 8,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 2,
            }),
        };

        let mapped = map_runtime_message(&message);
        assert_eq!(mapped.role, SaicodeMessageRole::Assistant);
        assert_eq!(
            mapped.blocks[0],
            SaicodeContentBlock::Text {
                text: "hello".to_string(),
            }
        );
        assert_eq!(
            mapped.blocks[1],
            SaicodeContentBlock::ToolUse {
                id: "tool-1".to_string(),
                name: "Read".to_string(),
                input: "{\"path\":\"/tmp/a\"}".to_string(),
            }
        );
        assert_eq!(mapped.usage.expect("usage").cache_read_input_tokens, 2);
    }

    #[test]
    fn snapshots_entire_session_for_ui_consumption() {
        let mut session = Session::new();
        session
            .push_message(ConversationMessage {
                role: MessageRole::User,
                blocks: vec![ContentBlock::Text {
                    text: "ping".to_string(),
                }],
                usage: None,
            })
            .expect("push message");

        let snapshot = snapshot_session(&session);
        assert_eq!(snapshot.session_id, session.session_id);
        assert_eq!(snapshot.message_count, 1);
        assert_eq!(snapshot.messages[0].role, SaicodeMessageRole::User);
    }

    #[test]
    fn builds_api_request_from_saicode_model_selection() {
        let request = build_message_request(SaicodeRequestEnvelope {
            selection: SaicodeModelSelection {
                model: "gpt-5.4".to_string(),
                effort: Some(SaicodeEffortLevel::Max),
            },
            max_tokens: 128,
            messages: vec![InputMessage::user_text("hello")],
            system: Some("system".to_string()),
            tools: None,
            tool_choice: None,
            stream: true,
        });

        assert_eq!(request.model, "gpt-5.4");
        assert_eq!(request.reasoning_effort, Some(api::ReasoningEffort::Max));
        assert!(request.stream);
    }
}
