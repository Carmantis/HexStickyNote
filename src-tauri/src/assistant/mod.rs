//! Assistant: an Ollama chat that can use notes, the calendar and HexTime
//! through tools. Changes are only made after the user confirms them.
//!
//! The backend keeps no conversation state. The frontend holds the messages
//! (in Ollama's chat format) and sends them with every call:
//!
//! 1. `send` runs the model. Read tools run straight away and their results go
//!    back to the model, until it answers in text or asks for a write tool.
//! 2. Write tools are not run; they come back as `pending` actions.
//! 3. `confirm` runs the approved actions (declined ones are reported to the
//!    model as declined) and continues the conversation.

pub mod tools;

use crate::local_model::OLLAMA_PREFIX;
use crate::ollama;
use crate::settings_manager::SettingsManager;
use chrono::Local;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::future::Future;
use thiserror::Error;
use tools::{ToolContext, ToolKind};

/// Model calls per user message before giving up (guards against tool loops)
const MAX_ROUNDS: usize = 8;

#[derive(Debug, Error)]
pub enum AssistantError {
    #[error("The assistant needs an Ollama model. Select one in Settings.")]
    NeedsOllamaModel,
    #[error("{0} does not support tools. Select an Ollama model with tool support (e.g. qwen3 or llama3.1).")]
    ModelWithoutTools(String),
    #[error("{0}")]
    Ollama(#[from] ollama::OllamaError),
    #[error("The assistant took too many steps without answering")]
    TooManySteps,
}

/// What the user is looking at, so "this note" and "today" mean the right thing
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AssistantContext {
    /// "notes", "calendar" or "time"
    pub view: Option<String>,
    /// The note open in the editor, if any
    pub open_note_id: Option<String>,
}

/// A write tool call waiting for the user's decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub call_id: Option<String>,
    pub tool: String,
    pub arguments: Value,
    /// What the action will do, in plain words
    pub summary: String,
}

/// The user's answer to a pending action
#[derive(Debug, Clone, Deserialize)]
pub struct Decision {
    #[serde(flatten)]
    pub action: PendingAction,
    pub approved: bool,
}

/// A tool that ran during a turn, for showing in the conversation
#[derive(Debug, Clone, Serialize)]
pub struct ActionRecord {
    pub tool: String,
    pub summary: String,
    pub ok: bool,
    pub error: Option<String>,
}

/// Result of one `send` or `confirm` call
#[derive(Debug, Serialize)]
pub struct AssistantTurn {
    /// The whole conversation, to send back with the next call
    pub messages: Vec<Value>,
    /// The assistant's text, if it wrote any
    pub reply: Option<String>,
    /// Write actions waiting for confirmation
    pub pending: Vec<PendingAction>,
    /// Tools that ran during this call
    pub actions: Vec<ActionRecord>,
}

/// One model call: conversation in, assistant message out
pub trait ChatBackend {
    fn chat(&self, messages: Vec<Value>, tools: Vec<Value>) -> impl Future<Output = Result<Value, AssistantError>> + Send;
}

/// The Ollama model selected in Settings
pub struct OllamaBackend {
    client: Client,
    model: String,
    /// Thinking models would otherwise spend the answer on reasoning
    disable_thinking: bool,
}

impl OllamaBackend {
    pub async fn new(client: &Client, settings: &SettingsManager) -> Result<Self, AssistantError> {
        let model = settings
            .get_active_model()
            .and_then(|id| id.strip_prefix(OLLAMA_PREFIX).map(str::to_string))
            .ok_or(AssistantError::NeedsOllamaModel)?;

        let capabilities = ollama::capabilities(client, &model).await?;
        if !capabilities.iter().any(|c| c == "tools") {
            return Err(AssistantError::ModelWithoutTools(model));
        }

        Ok(Self {
            client: client.clone(),
            disable_thinking: capabilities.iter().any(|c| c == "thinking"),
            model,
        })
    }
}

impl ChatBackend for OllamaBackend {
    async fn chat(&self, messages: Vec<Value>, tools: Vec<Value>) -> Result<Value, AssistantError> {
        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "stream": false,
        });
        if self.disable_thinking {
            body["think"] = json!(false);
        }
        Ok(ollama::chat(&self.client, body).await?)
    }
}

fn system_prompt(context: &AssistantContext) -> String {
    let now = Local::now();
    let mut prompt = format!(
        "You are the assistant in HexStickyNote, a desktop app with sticky notes, a calendar and \
time tracking (HexTime). Today is {today}; the local time is {time} (UTC{offset}).\n\
Use the tools to look things up instead of guessing. Dates are YYYY-MM-DD and times HH:MM \
(24-hour) in local time.\n\
When the user asks for a change (create an event or a note, start or stop the timer), call the tool \
directly: the app shows the action to the user for confirmation, so do not ask for confirmation yourself.\n\
If a tool returns an error, explain it briefly. Answer in the user's language, concisely. Markdown is allowed.",
        today = now.format("%A %Y-%m-%d"),
        time = now.format("%H:%M"),
        offset = now.format("%:z"),
    );

    if let Some(view) = context.view.as_deref() {
        prompt.push_str(&format!("\nThe user is looking at the {} view.", view));
    }
    let open_note = context.open_note_id.as_deref().and_then(|id| {
        crate::card_manager::get_all_cards().ok()?.into_iter().find(|c| c.id == id)
    });
    if let Some(note) = open_note {
        prompt.push_str(&format!(
            "\nThe user has the note “{}” (id {}) open; \"this note\" means that one. \
Read it with read_note before changing it, and change it with update_note.",
            crate::card_manager::extract_title_from_content(&note.content),
            note.id
        ));
    }
    prompt
}

/// What a read tool did, in plain words
fn read_summary(tool: &str) -> String {
    match tool {
        "list_events" => "Checked the calendar",
        "list_notes" => "Listed notes",
        "read_note" => "Read a note",
        "list_time_entries" => "Checked time entries",
        "get_timer" => "Checked the timer",
        other => other,
    }
    .to_string()
}

struct ToolCall {
    id: Option<String>,
    name: String,
    arguments: Value,
}

fn tool_calls(message: &Value) -> Vec<ToolCall> {
    message["tool_calls"]
        .as_array()
        .map(|calls| {
            calls
                .iter()
                .filter_map(|call| {
                    let function = &call["function"];
                    let arguments = match &function["arguments"] {
                        // Some models send the arguments as a JSON string
                        Value::String(s) => serde_json::from_str(s).unwrap_or(Value::Null),
                        other => other.clone(),
                    };
                    Some(ToolCall {
                        id: call["id"].as_str().map(str::to_string),
                        name: function["name"].as_str()?.to_string(),
                        arguments,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn tool_message(name: &str, call_id: Option<&str>, content: Value) -> Value {
    let mut message = json!({ "role": "tool", "tool_name": name, "content": content.to_string() });
    if let Some(id) = call_id {
        message["tool_call_id"] = json!(id);
    }
    message
}

fn text_of(message: &Value) -> Option<String> {
    message["content"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Run the model until it answers or asks for a write action
async fn run<B: ChatBackend>(
    backend: &B,
    ctx: &ToolContext<'_>,
    context: &AssistantContext,
    mut messages: Vec<Value>,
    mut actions: Vec<ActionRecord>,
) -> Result<AssistantTurn, AssistantError> {
    for _ in 0..MAX_ROUNDS {
        let mut request = vec![json!({ "role": "system", "content": system_prompt(context) })];
        request.extend(messages.iter().cloned());

        let reply = backend.chat(request, tools::definitions()).await?;
        messages.push(reply.clone());

        let calls = tool_calls(&reply);
        if calls.is_empty() {
            return Ok(AssistantTurn { reply: text_of(&reply), messages, pending: vec![], actions });
        }

        let mut pending = Vec::new();
        for call in calls {
            match tools::kind(&call.name) {
                Ok(ToolKind::Read) => {
                    let result = tools::execute(ctx, &call.name, &call.arguments).await;
                    let (content, ok, error) = match result {
                        Ok(value) => (value, true, None),
                        Err(e) => (json!({ "error": e.to_string() }), false, Some(e.to_string())),
                    };
                    messages.push(tool_message(&call.name, call.id.as_deref(), content));
                    actions.push(ActionRecord { summary: read_summary(&call.name), tool: call.name, ok, error });
                }
                Ok(ToolKind::Write) => match tools::describe(&call.name, &call.arguments) {
                    Ok(summary) => pending.push(PendingAction {
                        call_id: call.id,
                        tool: call.name,
                        arguments: call.arguments,
                        summary,
                    }),
                    // Bad arguments go back to the model to fix, not to the user
                    Err(e) => messages.push(tool_message(&call.name, call.id.as_deref(), json!({ "error": e.to_string() }))),
                },
                Err(e) => messages.push(tool_message(&call.name, call.id.as_deref(), json!({ "error": e.to_string() }))),
            }
        }

        if !pending.is_empty() {
            return Ok(AssistantTurn { reply: text_of(&reply), messages, pending, actions });
        }
    }

    Err(AssistantError::TooManySteps)
}

/// Continue the conversation after the user wrote a message (already appended to `messages`)
pub async fn send<B: ChatBackend>(
    backend: &B,
    ctx: &ToolContext<'_>,
    context: &AssistantContext,
    messages: Vec<Value>,
) -> Result<AssistantTurn, AssistantError> {
    run(backend, ctx, context, messages, Vec::new()).await
}

/// Apply the user's decisions on pending actions and continue the conversation
pub async fn confirm<B: ChatBackend>(
    backend: &B,
    ctx: &ToolContext<'_>,
    context: &AssistantContext,
    mut messages: Vec<Value>,
    decisions: Vec<Decision>,
) -> Result<AssistantTurn, AssistantError> {
    let mut actions = Vec::new();

    for decision in decisions {
        let action = decision.action;
        let (content, ok, error) = if !decision.approved {
            (
                json!({ "declined": true, "message": "The user declined this action. It was not performed." }),
                false,
                Some("Declined".to_string()),
            )
        } else if tools::kind(&action.tool).ok() != Some(ToolKind::Write) {
            (json!({ "error": "not a confirmable action" }), false, Some("Not allowed".to_string()))
        } else {
            match tools::execute(ctx, &action.tool, &action.arguments).await {
                Ok(value) => (value, true, None),
                Err(e) => (json!({ "error": e.to_string() }), false, Some(e.to_string())),
            }
        };

        messages.push(tool_message(&action.tool, action.call_id.as_deref(), content));
        actions.push(ActionRecord { tool: action.tool, summary: action.summary, ok, error });
    }

    run(backend, ctx, context, messages, actions).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::db::{queries, DbPool};
    use crate::hextime::HexTime;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    /// Plays back scripted assistant messages and records what it was sent
    struct ScriptedModel {
        replies: Mutex<VecDeque<Value>>,
        seen: Mutex<Vec<Vec<Value>>>,
    }

    impl ScriptedModel {
        fn new(replies: Vec<Value>) -> Self {
            Self { replies: Mutex::new(replies.into()), seen: Mutex::new(Vec::new()) }
        }
    }

    impl ChatBackend for ScriptedModel {
        async fn chat(&self, messages: Vec<Value>, _tools: Vec<Value>) -> Result<Value, AssistantError> {
            self.seen.lock().unwrap().push(messages);
            Ok(self
                .replies
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| json!({ "role": "assistant", "content": "(script ended)" })))
        }
    }

    fn call(id: &str, name: &str, arguments: Value) -> Value {
        json!({
            "role": "assistant",
            "content": "",
            "tool_calls": [{ "id": id, "function": { "name": name, "arguments": arguments } }]
        })
    }

    fn text(content: &str) -> Value {
        json!({ "role": "assistant", "content": content })
    }

    fn user(content: &str) -> Vec<Value> {
        vec![json!({ "role": "user", "content": content })]
    }

    struct Fixture {
        db: DbPool,
        hextime: HexTime,
        client: Client,
    }

    impl Fixture {
        fn new() -> Self {
            Self { db: DbPool::open_in_memory().unwrap(), hextime: HexTime::default(), client: Client::new() }
        }

        fn ctx(&self) -> ToolContext<'_> {
            ToolContext { calendar: Some(&self.db), hextime: &self.hextime, client: &self.client }
        }

        fn event_count(&self) -> usize {
            queries::query_events_in_range(&self.db, 0, i64::MAX).unwrap().len()
        }
    }

    #[tokio::test]
    async fn runs_read_tools_and_returns_the_answer() {
        let f = Fixture::new();
        tools::execute(&f.ctx(), "create_event", &json!({ "title": "Dentist", "date": "2026-10-05", "start_time": "10:00" }))
            .await
            .unwrap();

        let model = ScriptedModel::new(vec![
            call("c1", "list_events", json!({ "from_date": "2026-10-05" })),
            text("You have Dentist at 10:00."),
        ]);
        let turn = send(&model, &f.ctx(), &AssistantContext::default(), user("What do I have on Monday?")).await.unwrap();

        assert_eq!(turn.reply.as_deref(), Some("You have Dentist at 10:00."));
        assert!(turn.pending.is_empty());
        assert_eq!(turn.actions.len(), 1);
        assert!(turn.actions[0].ok);

        // The second model call saw the tool result, after the system prompt
        let seen = model.seen.lock().unwrap();
        assert_eq!(seen[1][0]["role"], "system");
        let tool_result = seen[1].iter().find(|m| m["role"] == "tool").unwrap();
        assert_eq!(tool_result["tool_call_id"], "c1");
        assert!(tool_result["content"].as_str().unwrap().contains("Dentist"));
    }

    #[tokio::test]
    async fn holds_write_tools_until_confirmed() {
        let f = Fixture::new();
        let model = ScriptedModel::new(vec![
            call("c1", "create_event", json!({ "title": "Gym", "date": "2026-10-06", "start_time": "18:00" })),
            text("Added Gym on Tuesday."),
        ]);

        let turn = send(&model, &f.ctx(), &AssistantContext::default(), user("Add gym on Tuesday at 18")).await.unwrap();
        assert_eq!(turn.pending.len(), 1);
        assert_eq!(turn.pending[0].summary, "Create event “Gym” on Tue 6 Oct 2026, 18:00–19:00");
        assert_eq!(f.event_count(), 0, "nothing is written before confirmation");

        let decision = Decision { action: turn.pending[0].clone(), approved: true };
        let turn = confirm(&model, &f.ctx(), &AssistantContext::default(), turn.messages, vec![decision]).await.unwrap();
        assert_eq!(f.event_count(), 1);
        assert!(turn.actions[0].ok);
        assert_eq!(turn.reply.as_deref(), Some("Added Gym on Tuesday."));
    }

    #[tokio::test]
    async fn tells_the_model_about_declined_actions() {
        let f = Fixture::new();
        let model = ScriptedModel::new(vec![
            call("c1", "create_event", json!({ "title": "Gym", "date": "2026-10-06" })),
            text("Okay, I did not add it."),
        ]);

        let turn = send(&model, &f.ctx(), &AssistantContext::default(), user("Add gym")).await.unwrap();
        let decision = Decision { action: turn.pending[0].clone(), approved: false };
        let turn = confirm(&model, &f.ctx(), &AssistantContext::default(), turn.messages, vec![decision]).await.unwrap();

        assert_eq!(f.event_count(), 0);
        assert!(!turn.actions[0].ok);
        let tool_result = turn.messages.iter().rev().find(|m| m["role"] == "tool").unwrap();
        assert!(tool_result["content"].as_str().unwrap().contains("declined"));
    }

    #[tokio::test]
    async fn sends_bad_write_arguments_back_to_the_model() {
        let f = Fixture::new();
        let model = ScriptedModel::new(vec![
            call("c1", "create_event", json!({ "title": "Gym", "date": "next tuesday" })),
            call("c2", "create_event", json!({ "title": "Gym", "date": "2026-10-06" })),
        ]);

        let turn = send(&model, &f.ctx(), &AssistantContext::default(), user("Add gym")).await.unwrap();
        // The invalid call never reached the user; the corrected one did
        assert_eq!(turn.pending.len(), 1);
        assert_eq!(turn.pending[0].call_id.as_deref(), Some("c2"));
        let error = turn.messages.iter().find(|m| m["role"] == "tool").unwrap();
        assert!(error["content"].as_str().unwrap().contains("YYYY-MM-DD"));
    }

    #[tokio::test]
    async fn refuses_to_confirm_read_or_unknown_tools() {
        let f = Fixture::new();
        let model = ScriptedModel::new(vec![text("Done.")]);
        let forged = PendingAction { call_id: None, tool: "delete_everything".into(), arguments: Value::Null, summary: "x".into() };

        let turn = confirm(&model, &f.ctx(), &AssistantContext::default(), user("hi"), vec![Decision { action: forged, approved: true }]).await.unwrap();
        assert!(!turn.actions[0].ok);
    }

    #[tokio::test]
    async fn stops_a_model_stuck_in_a_tool_loop() {
        let f = Fixture::new();
        let model = ScriptedModel::new(
            (0..MAX_ROUNDS + 1).map(|i| call(&format!("c{i}"), "list_notes_typo", json!({}))).collect(),
        );
        let result = send(&model, &f.ctx(), &AssistantContext::default(), user("loop")).await;
        assert!(matches!(result, Err(AssistantError::TooManySteps)));
    }

    /// Needs a running Ollama with a tool-capable model (ASSISTANT_TEST_MODEL, default qwen3.8:27b)
    #[tokio::test]
    #[ignore]
    async fn real_model_creates_and_finds_an_event() {
        let f = Fixture::new();
        let model = std::env::var("ASSISTANT_TEST_MODEL").unwrap_or_else(|_| "qwen3.8:27b".to_string());
        let caps = ollama::capabilities(&f.client, &model).await.unwrap();
        let backend = OllamaBackend { client: f.client.clone(), disable_thinking: caps.iter().any(|c| c == "thinking"), model };
        let tomorrow = (Local::now() + chrono::Duration::days(1)).format("%Y-%m-%d").to_string();

        let turn = send(&backend, &f.ctx(), &AssistantContext::default(), user("Lisää huomiselle hammaslääkäri klo 10")).await.unwrap();
        println!("pending: {:?}\nreply: {:?}", turn.pending, turn.reply);
        assert_eq!(turn.pending.len(), 1);
        assert_eq!(turn.pending[0].tool, "create_event");
        assert_eq!(turn.pending[0].arguments["date"], tomorrow.as_str());

        let decision = Decision { action: turn.pending[0].clone(), approved: true };
        let turn = confirm(&backend, &f.ctx(), &AssistantContext::default(), turn.messages, vec![decision]).await.unwrap();
        println!("after confirm: {:?}", turn.reply);
        assert_eq!(f.event_count(), 1);

        let mut messages = turn.messages;
        messages.push(json!({ "role": "user", "content": "Mitä minulla on huomenna?" }));
        let turn = send(&backend, &f.ctx(), &AssistantContext::default(), messages).await.unwrap();
        println!("actions: {:?}\nreply: {:?}", turn.actions, turn.reply);
        assert!(turn.actions.iter().any(|a| a.tool == "list_events"));
        assert!(turn.reply.unwrap_or_default().to_lowercase().contains("hammaslääkäri"));
    }

    #[test]
    fn parses_string_arguments() {
        let message = json!({
            "tool_calls": [{ "function": { "name": "list_events", "arguments": "{\"from_date\":\"2026-10-05\"}" } }]
        });
        let calls = tool_calls(&message);
        assert_eq!(calls[0].arguments["from_date"], "2026-10-05");
        assert_eq!(calls[0].id, None);
    }
}
