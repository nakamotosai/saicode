use std::collections::BTreeMap;
use std::env;
use std::sync::Arc;

use api::OutputContentBlock;
use serde_json::json;

use super::agent_runtime::push_output_block;
use super::execute_tool;
use super::test_support::{env_lock, HttpResponse, TestServer};

struct ScopedEnvVar {
    key: &'static str,
    previous: Option<String>,
}

impl ScopedEnvVar {
    fn set(key: &'static str, value: impl Into<String>) -> Self {
        let previous = env::var(key).ok();
        env::set_var(key, value.into());
        Self { key, previous }
    }
}

impl Drop for ScopedEnvVar {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            env::set_var(self.key, previous);
        } else {
            env::remove_var(self.key);
        }
    }
}

fn has_webfetch_provider_credentials() -> bool {
    for key in ["CPA_API_KEY", "CLIPROXYAPI_API_KEY", "OPENAI_API_KEY"] {
        if env::var(key).ok().filter(|value| !value.trim().is_empty()).is_some() {
            return true;
        }
    }

    let Some(home) = env::var_os("HOME") else {
        return false;
    };
    let config_path = std::path::Path::new(&home).join(".saicode/config.json");
    let Ok(raw) = std::fs::read_to_string(config_path) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    ["cpa", "cliproxyapi"].iter().any(|provider| {
        json.get("providers")
            .and_then(|providers| providers.get(*provider))
            .and_then(|provider| provider.get("apiKey").or_else(|| provider.get("api_key")))
            .and_then(serde_json::Value::as_str)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    })
}

#[test]
fn web_fetch_returns_prompt_aware_summary() {
    if !has_webfetch_provider_credentials() {
        return;
    }
    let server = TestServer::spawn(Arc::new(|request_line: &str| {
        assert!(request_line.starts_with("GET /page "));
        HttpResponse::html(
            200,
            "OK",
            "<html><head><title>Ignored</title></head><body><h1>Test Page</h1><p>Hello <b>world</b> from local server.</p></body></html>",
        )
    }));

    let result = execute_tool(
        "WebFetch",
        &json!({
            "url": format!("http://{}/page", server.addr()),
            "prompt": "Summarize this page"
        }),
    )
    .expect("WebFetch should succeed");

    assert!(!result.trim().is_empty());

    let titled = execute_tool(
        "WebFetch",
        &json!({
            "url": format!("http://{}/page", server.addr()),
            "prompt": "What is the page title?"
        }),
    )
    .expect("WebFetch title query should succeed");
    assert!(!titled.trim().is_empty());
}

#[test]
fn web_fetch_supports_plain_text_and_rejects_invalid_url() {
    if !has_webfetch_provider_credentials() {
        return;
    }
    let server = TestServer::spawn(Arc::new(|request_line: &str| {
        assert!(request_line.starts_with("GET /plain "));
        HttpResponse::text(200, "OK", "plain text response")
    }));

    let result = execute_tool(
        "WebFetch",
        &json!({
            "url": format!("http://{}/plain", server.addr()),
            "prompt": "Show me the content"
        }),
    )
    .expect("WebFetch should succeed for text content");

    assert!(!result.trim().is_empty());

    let error = execute_tool(
        "WebFetch",
        &json!({
            "url": "not a url",
            "prompt": "Summarize"
        }),
    )
    .expect_err("invalid URL should fail");
    assert!(error.contains("valid") || error.contains("invalid"));
}

#[test]
fn web_search_extracts_and_filters_results() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let server = TestServer::spawn(Arc::new(|request_line: &str| {
        assert!(request_line.starts_with("GET /search?"));
        assert!(request_line.contains("q=rust"));
        assert!(request_line.contains("web"));
        assert!(request_line.contains("search"));
        HttpResponse::html(
            200,
            "OK",
            r#"
            <html><body>
              <a class="result__a" href="https://docs.rs/reqwest">Reqwest docs</a>
              <a class="result__a" href="https://example.com/blocked">Blocked result</a>
            </body></html>
            "#,
        )
    }));

    let search_base_url = format!("http://{}/search", server.addr());
    let _search_base_url =
        ScopedEnvVar::set("SAICODE_WEB_SEARCH_BASE_URL", search_base_url.clone());
    let _legacy_search_base_url = ScopedEnvVar::set("CLAWD_WEB_SEARCH_BASE_URL", search_base_url);
    let _disable_local = ScopedEnvVar::set("SAICODE_DISABLE_LOCAL_SAI_SEARCH", "1");
    let _disable_ssh = ScopedEnvVar::set("SAICODE_DISABLE_SAI_SEARCH_SSH", "1");
    let result = execute_tool(
        "WebSearch",
        &json!({
            "query": "rust web search",
            "allowed_domains": ["https://DOCS.rs/"],
            "blocked_domains": ["HTTPS://EXAMPLE.COM"]
        }),
    )
    .expect("WebSearch should succeed");

    assert!(result.contains("https://docs.rs/reqwest"));
    assert!(!result.contains("https://example.com/blocked"));
}

#[test]
fn web_search_handles_generic_links_and_invalid_base_url() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let server = TestServer::spawn(Arc::new(|request_line: &str| {
        assert!(request_line.starts_with("GET /fallback?"));
        assert!(request_line.contains("q=generic"));
        assert!(request_line.contains("links"));
        HttpResponse::html(
            200,
            "OK",
            r#"
            <html><body>
              <a href="https://example.com/one">Example One</a>
              <a href="https://example.com/one">Duplicate Example One</a>
              <a href="https://docs.rs/tokio">Tokio Docs</a>
            </body></html>
            "#,
        )
    }));

    let fallback_base_url = format!("http://{}/fallback", server.addr());
    let _fallback_base_url =
        ScopedEnvVar::set("SAICODE_WEB_SEARCH_BASE_URL", fallback_base_url.clone());
    let _legacy_fallback_base_url =
        ScopedEnvVar::set("CLAWD_WEB_SEARCH_BASE_URL", fallback_base_url);
    let _disable_local = ScopedEnvVar::set("SAICODE_DISABLE_LOCAL_SAI_SEARCH", "1");
    let _disable_ssh = ScopedEnvVar::set("SAICODE_DISABLE_SAI_SEARCH_SSH", "1");
    let result = execute_tool("WebSearch", &json!({ "query": "generic links" }))
        .expect("WebSearch fallback parsing should succeed");

    assert!(result.contains("https://example.com/one"));
    assert!(result.contains("https://docs.rs/tokio"));

    drop(_fallback_base_url);
    drop(_legacy_fallback_base_url);
    let _invalid_base_url = ScopedEnvVar::set("CLAWD_WEB_SEARCH_BASE_URL", "://bad-base-url");
    let error = execute_tool("WebSearch", &json!({ "query": "generic links" }))
        .expect_err("invalid base URL should fail");
    assert!(error.contains("failed") || error.contains("invalid") || error.contains("could not"));
}

#[test]
fn pending_tools_preserve_multiple_streaming_tool_calls_by_index() {
    let mut events = Vec::new();
    let mut pending_tools = BTreeMap::new();

    push_output_block(
        OutputContentBlock::ToolUse {
            id: "tool-1".to_string(),
            name: "read_file".to_string(),
            input: json!({}),
        },
        1,
        &mut events,
        &mut pending_tools,
        true,
    );
    push_output_block(
        OutputContentBlock::ToolUse {
            id: "tool-2".to_string(),
            name: "grep_search".to_string(),
            input: json!({}),
        },
        2,
        &mut events,
        &mut pending_tools,
        true,
    );

    pending_tools
        .get_mut(&1)
        .expect("first tool pending")
        .2
        .push_str("{\"path\":\"src/main.rs\"}");
    pending_tools
        .get_mut(&2)
        .expect("second tool pending")
        .2
        .push_str("{\"pattern\":\"TODO\"}");

    assert_eq!(
        pending_tools.remove(&1),
        Some((
            "tool-1".to_string(),
            "read_file".to_string(),
            "{\"path\":\"src/main.rs\"}".to_string(),
        ))
    );
    assert_eq!(
        pending_tools.remove(&2),
        Some((
            "tool-2".to_string(),
            "grep_search".to_string(),
            "{\"pattern\":\"TODO\"}".to_string(),
        ))
    );
}
