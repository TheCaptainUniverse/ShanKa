use crate::{config, persona, selection::SelectionMode};
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub trait RewriteProvider {
    fn name(&self) -> &'static str;
    fn rewrite(&self, request: RewriteRequest<'_>) -> Result<RewriteResponse, RewriteError>;
}

pub struct RewriteRequest<'a> {
    pub text: &'a str,
    pub mode: SelectionMode,
    pub persona: Option<persona::ResolvedPersona>,
}

#[derive(Debug)]
pub struct RewriteResponse {
    pub text: String,
    pub provider: &'static str,
    pub duration: Duration,
}

#[derive(Debug)]
pub enum RewriteError {
    Config(String),
    EmptyInput,
    Timeout,
    InvalidResponse(String),
    Remote(String),
}

impl std::fmt::Display for RewriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(message) => write!(formatter, "rewrite config error: {message}"),
            Self::EmptyInput => write!(formatter, "rewrite input is empty"),
            Self::Timeout => write!(formatter, "remote rewrite error: request timed out"),
            Self::InvalidResponse(message) => {
                write!(formatter, "remote rewrite response invalid: {message}")
            }
            Self::Remote(message) => write!(formatter, "remote rewrite error: {message}"),
        }
    }
}

impl std::error::Error for RewriteError {}

struct MockRewriteProvider;
struct OpenAiCompatibleRewriteProvider {
    settings: config::AppSettingsConfig,
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

#[derive(Deserialize)]
struct ChatCompletionChoice {
    message: ChatCompletionMessage,
}

#[derive(Deserialize)]
struct ChatCompletionMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct StructuredRewriteContent {
    text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputMode {
    Json,
    Plain,
}

#[derive(Debug)]
struct ProviderCallError {
    message: String,
    retry_plain: bool,
    retry_same: bool,
    timeout: bool,
    invalid_response: bool,
}

impl RewriteProvider for MockRewriteProvider {
    fn name(&self) -> &'static str {
        "mock-rewrite"
    }

    fn rewrite(&self, request: RewriteRequest<'_>) -> Result<RewriteResponse, RewriteError> {
        let started_at = Instant::now();
        let normalized = normalize_text(request.text);

        if normalized.is_empty() {
            return Err(RewriteError::EmptyInput);
        }

        let label = match request.mode {
            SelectionMode::Safe => "Shanka Safe Mock",
            SelectionMode::Magic => "Shanka Magic Mock",
        };
        let persona_label = request
            .persona
            .as_ref()
            .map(|persona| format!(" / {}", persona.name))
            .unwrap_or_default();

        Ok(RewriteResponse {
            text: format!("[{label}{persona_label}]\n{normalized}"),
            provider: self.name(),
            duration: started_at.elapsed(),
        })
    }
}

impl RewriteProvider for OpenAiCompatibleRewriteProvider {
    fn name(&self) -> &'static str {
        "openai-compatible"
    }

    fn rewrite(&self, request: RewriteRequest<'_>) -> Result<RewriteResponse, RewriteError> {
        let started_at = Instant::now();
        let input = normalize_text(request.text);
        if input.is_empty() {
            return Err(RewriteError::EmptyInput);
        }

        let output = tauri::async_runtime::block_on(call_chat_completion_with_json_fallback(
            &self.settings,
            request.mode,
            request.persona,
            &input,
        ))?;

        Ok(RewriteResponse {
            text: output,
            provider: self.name(),
            duration: started_at.elapsed(),
        })
    }
}

pub fn setup(_app: &tauri::AppHandle) -> tauri::Result<()> {
    println!("[rewrite] provider ready: openai-compatible with mock fallback");
    Ok(())
}

pub fn rewrite_selected_text(
    app: &tauri::AppHandle,
    mode: SelectionMode,
    text: &str,
) -> Result<RewriteResponse, RewriteError> {
    rewrite_selected_text_with_persona(app, mode, text, None)
}

pub fn rewrite_selected_text_with_persona(
    app: &tauri::AppHandle,
    mode: SelectionMode,
    text: &str,
    persona_id: Option<&str>,
) -> Result<RewriteResponse, RewriteError> {
    let config =
        config::load_or_create(app).map_err(|error| RewriteError::Config(error.to_string()))?;
    let settings = config
        .settings
        .with_resolved_api_key()
        .map_err(|error| RewriteError::Config(error.to_string()))?;
    let persona = persona_id
        .and_then(|persona_id| persona::resolve_persona(&config.personas, Some(persona_id)));

    if settings.can_use_remote_provider() {
        let provider = OpenAiCompatibleRewriteProvider { settings };
        return provider.rewrite(RewriteRequest {
            text,
            mode,
            persona,
        });
    }

    let provider = MockRewriteProvider;
    provider.rewrite(RewriteRequest {
        text,
        mode,
        persona,
    })
}

pub fn default_safe_persona_id() -> &'static str {
    persona::default_safe_persona_id()
}

pub fn test_provider_connection(settings: config::AppSettingsConfig) -> Result<(), RewriteError> {
    let settings = settings
        .with_resolved_api_key()
        .map_err(|error| RewriteError::Config(error.to_string()))?;

    if !settings.can_use_remote_provider() {
        return Err(RewriteError::Config(
            "api_key, base_url, and model are required".to_string(),
        ));
    }

    tauri::async_runtime::block_on(call_provider_connection_test(&settings))
}

fn normalize_text(text: &str) -> String {
    text.trim_matches('\0')
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

async fn call_chat_completion_with_json_fallback(
    settings: &config::AppSettingsConfig,
    mode: SelectionMode,
    persona: Option<persona::ResolvedPersona>,
    text: &str,
) -> Result<String, RewriteError> {
    if preferred_output_mode(settings) == OutputMode::Plain {
        return call_chat_completion_with_retries(settings, mode, persona, text, OutputMode::Plain)
            .await
            .map_err(ProviderCallError::into_rewrite_error);
    }

    match call_chat_completion_with_retries(settings, mode, persona.clone(), text, OutputMode::Json)
        .await
    {
        Ok(output) => Ok(output),
        Err(error) if error.timeout => Err(RewriteError::Timeout),
        Err(error) if error.retry_plain => {
            println!(
                "[rewrite] JSON mode failed; retrying plain content mode: {}",
                error.message
            );
            call_chat_completion_with_retries(settings, mode, persona, text, OutputMode::Plain)
                .await
                .map_err(ProviderCallError::into_rewrite_error)
        }
        Err(error) => Err(error.into_rewrite_error()),
    }
}

async fn call_chat_completion_with_retries(
    settings: &config::AppSettingsConfig,
    mode: SelectionMode,
    persona: Option<persona::ResolvedPersona>,
    text: &str,
    output_mode: OutputMode,
) -> Result<String, ProviderCallError> {
    let max_attempts = 2;

    for attempt in 1..=max_attempts {
        match call_chat_completion(settings, mode, persona.clone(), text, output_mode).await {
            Ok(output) => return Ok(output),
            Err(error)
                if error.retry_same
                    && !error.timeout
                    && attempt < max_attempts
                    && !error.retry_plain =>
            {
                println!(
                    "[rewrite] provider {} mode attempt {attempt}/{max_attempts} failed; retrying: {}",
                    output_mode.label(),
                    error.message
                );
                tokio::time::sleep(Duration::from_millis(180)).await;
            }
            Err(error) => return Err(error),
        }
    }

    unreachable!("provider retry loop always returns before exhausting attempts")
}

async fn call_chat_completion(
    settings: &config::AppSettingsConfig,
    mode: SelectionMode,
    persona: Option<persona::ResolvedPersona>,
    text: &str,
    output_mode: OutputMode,
) -> Result<String, ProviderCallError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(settings.timeout_ms))
        .build()
        .map_err(|error| {
            ProviderCallError::remote(format!("failed to create HTTP client: {error}"))
        })?;
    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );

    let response = client
        .post(endpoint)
        .bearer_auth(&settings.api_key)
        .header(header::ACCEPT_ENCODING, "identity")
        .json(&ChatCompletionRequest {
            model: settings.model.clone(),
            messages: rewrite_messages(mode, persona, text, output_mode),
            temperature: match mode {
                SelectionMode::Safe => 0.2,
                SelectionMode::Magic => 0.7,
            },
            max_tokens: max_tokens_for_text(text),
            response_format: match output_mode {
                OutputMode::Json => Some(ResponseFormat {
                    kind: "json_object",
                }),
                OutputMode::Plain => None,
            },
        })
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                ProviderCallError::timeout()
            } else {
                ProviderCallError::retryable_remote(format!("request failed: {error}"))
            }
        })?;

    let status = response.status();
    let body = response.bytes().await.map_err(|error| {
        ProviderCallError::retryable_remote(format!(
            "failed to read provider response body: {error}"
        ))
    })?;

    if !status.is_success() {
        let body = response_body_sample(&body);
        return Err(ProviderCallError {
            message: format!(
                "provider returned HTTP {status}{}",
                provider_text_debug_suffix("body sample", &body)
            ),
            retry_plain: matches!(output_mode, OutputMode::Json)
                && response_format_may_be_unsupported(status.as_u16(), &body),
            retry_same: response_status_is_transient(status.as_u16()),
            timeout: false,
            invalid_response: false,
        });
    }

    let payload = serde_json::from_slice::<ChatCompletionResponse>(&body).map_err(|error| {
        ProviderCallError::retryable_invalid_response(format!(
            "failed to parse provider response JSON: {error}{}",
            provider_body_debug_suffix(&body)
        ))
    })?;
    let output = payload
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
        .map(|content| content.trim().to_string())
        .unwrap_or_default();

    let output = match output_mode {
        OutputMode::Json => parse_structured_rewrite(&output)?,
        OutputMode::Plain => output,
    };

    if output.trim().is_empty() {
        return Err(ProviderCallError {
            message: "provider returned an empty rewrite".to_string(),
            retry_plain: matches!(output_mode, OutputMode::Json),
            retry_same: matches!(output_mode, OutputMode::Plain),
            timeout: false,
            invalid_response: true,
        });
    }

    Ok(output)
}

async fn call_provider_connection_test(
    settings: &config::AppSettingsConfig,
) -> Result<(), RewriteError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(settings.timeout_ms))
        .build()
        .map_err(|error| RewriteError::Remote(format!("PROVIDER_TEST_NETWORK: {error}")))?;
    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );

    let response = client
        .post(endpoint)
        .bearer_auth(&settings.api_key)
        .header(header::ACCEPT_ENCODING, "identity")
        .json(&ChatCompletionRequest {
            model: settings.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: "Reply with OK.".to_string(),
                },
                ChatMessage {
                    role: "user",
                    content: "ping".to_string(),
                },
            ],
            temperature: 0.0,
            max_tokens: 8,
            response_format: None,
        })
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                RewriteError::Timeout
            } else {
                RewriteError::Remote(format!("PROVIDER_TEST_NETWORK: {error}"))
            }
        })?;

    let status = response.status();
    let body = response.bytes().await.map_err(|error| {
        RewriteError::Remote(format!(
            "PROVIDER_TEST_REMOTE: failed to read response: {error}"
        ))
    })?;

    if status.is_success() {
        return Ok(());
    }

    let body_sample = response_body_sample(&body);
    let category = provider_test_error_category(status.as_u16(), &body_sample);
    Err(RewriteError::Remote(format!(
        "{category}{}",
        provider_text_debug_suffix("body sample", &body_sample)
    )))
}

impl ProviderCallError {
    fn timeout() -> Self {
        Self {
            message: "request timed out".to_string(),
            retry_plain: false,
            retry_same: false,
            timeout: true,
            invalid_response: false,
        }
    }

    fn remote(message: String) -> Self {
        Self {
            message,
            retry_plain: false,
            retry_same: false,
            timeout: false,
            invalid_response: false,
        }
    }

    fn retryable_remote(message: String) -> Self {
        Self {
            message,
            retry_plain: false,
            retry_same: true,
            timeout: false,
            invalid_response: false,
        }
    }

    fn retryable_invalid_response(message: String) -> Self {
        Self {
            message,
            retry_plain: false,
            retry_same: true,
            timeout: false,
            invalid_response: true,
        }
    }

    fn into_rewrite_error(self) -> RewriteError {
        if self.timeout {
            RewriteError::Timeout
        } else if self.invalid_response {
            RewriteError::InvalidResponse(self.message)
        } else {
            RewriteError::Remote(self.message)
        }
    }
}

fn parse_structured_rewrite(content: &str) -> Result<String, ProviderCallError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(ProviderCallError {
            message: "provider returned empty JSON content".to_string(),
            retry_plain: true,
            retry_same: false,
            timeout: false,
            invalid_response: true,
        });
    }

    parse_structured_rewrite_payload(content).map_err(|error| ProviderCallError {
        message: format!(
            "failed to parse rewrite JSON content: {error}{}",
            provider_text_debug_suffix("content sample", content)
        ),
        retry_plain: true,
        retry_same: false,
        timeout: false,
        invalid_response: true,
    })
}

fn parse_structured_rewrite_payload(content: &str) -> Result<String, serde_json::Error> {
    serde_json::from_str::<StructuredRewriteContent>(content)
        .or_else(|_| {
            extract_json_object(content)
                .map(|json| serde_json::from_str::<StructuredRewriteContent>(json))
                .unwrap_or_else(|| serde_json::from_str::<StructuredRewriteContent>(content))
        })
        .map(|payload| payload.text.trim().to_string())
}

fn response_format_may_be_unsupported(status: u16, body: &str) -> bool {
    if status != 400 && status != 422 {
        return false;
    }

    let body = body.to_ascii_lowercase();
    body.contains("response_format")
        || body.contains("json_object")
        || body.contains("unsupported")
        || body.contains("unknown")
        || body.contains("invalid")
}

fn response_status_is_transient(status: u16) -> bool {
    matches!(status, 408 | 409 | 425 | 429 | 500 | 502 | 503 | 504)
}

fn provider_test_error_category(status: u16, body: &str) -> &'static str {
    let body = body.to_ascii_lowercase();

    if status == 401 || status == 403 {
        return "PROVIDER_TEST_AUTH";
    }

    if status == 404
        || body.contains("model")
        || body.contains("not found")
        || body.contains("does not exist")
    {
        return "PROVIDER_TEST_MODEL";
    }

    "PROVIDER_TEST_REMOTE"
}

fn max_tokens_for_text(text: &str) -> u32 {
    let approximate_input_tokens = text.chars().count().saturating_div(2) as u32;
    approximate_input_tokens.clamp(256, 4_096)
}

fn response_body_sample(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let sample = text.trim().chars().take(500).collect::<String>();
    let sample = redact_sensitive_sample(&sample);

    if sample.is_empty() {
        "<empty body>".to_string()
    } else {
        sample
    }
}

fn preferred_output_mode(settings: &config::AppSettingsConfig) -> OutputMode {
    if json_mode_is_risky_for_provider(settings) {
        OutputMode::Plain
    } else {
        OutputMode::Json
    }
}

fn json_mode_is_risky_for_provider(settings: &config::AppSettingsConfig) -> bool {
    let provider = settings.provider.trim().to_ascii_lowercase();
    let base_url = settings.base_url.trim().to_ascii_lowercase();

    provider == "deepseek" || base_url.contains("api.deepseek.com")
}

fn provider_body_debug_suffix(body: &[u8]) -> String {
    if !config::debug_logging_enabled() {
        return String::new();
    }

    provider_text_debug_suffix("body sample", &response_body_sample(body))
}

fn provider_text_debug_suffix(label: &str, text: &str) -> String {
    if !config::debug_logging_enabled() {
        return String::new();
    }

    let sample = text.trim().chars().take(500).collect::<String>();
    let sample = redact_sensitive_sample(&sample);
    if sample.is_empty() {
        format!("; {label}: <empty>")
    } else {
        format!("; {label}: {sample}")
    }
}

fn redact_sensitive_sample(sample: &str) -> String {
    let mut redacted = String::with_capacity(sample.len());
    let chars = sample.chars().collect::<Vec<_>>();
    let mut index = 0;

    while index < chars.len() {
        if starts_with_at(&chars, index, "sk-") {
            redacted.push_str("[redacted-api-key]");
            index += 3;
            while index < chars.len() && !is_secret_delimiter(chars[index]) {
                index += 1;
            }
            continue;
        }

        redacted.push(chars[index]);
        index += 1;
    }

    redacted
}

fn starts_with_at(chars: &[char], index: usize, needle: &str) -> bool {
    needle.chars().enumerate().all(|(offset, character)| {
        chars
            .get(index + offset)
            .is_some_and(|value| *value == character)
    })
}

fn extract_json_object(content: &str) -> Option<&str> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;

    if start >= end {
        return None;
    }

    Some(&content[start..=end])
}

fn is_secret_delimiter(character: char) -> bool {
    character.is_whitespace()
        || matches!(
            character,
            '"' | '\'' | ',' | ';' | ':' | '}' | ']' | ')' | '<' | '>'
        )
}

fn rewrite_messages(
    mode: SelectionMode,
    persona: Option<persona::ResolvedPersona>,
    text: &str,
    output_mode: OutputMode,
) -> Vec<ChatMessage> {
    let base_instruction = match mode {
        SelectionMode::Safe if matches!(output_mode, OutputMode::Json) => {
            "You are Shanka Safe Mode. Rewrite the user's selected text to be clear, natural, and grammatically correct while preserving the original meaning, tone, language, formatting, and approximate length. Return only valid json in this exact shape: {\"text\":\"rewritten text\"}. Do not include markdown, comments, or any extra keys.\n\nEXAMPLE INPUT:\n组织校内学术分享、职业规划讲座与跨院交流活动共15场\nEXAMPLE JSON OUTPUT:\n{\"text\":\"组织校内学术分享、职业规划讲座与跨院交流活动共 15 场\"}"
        }
        SelectionMode::Magic if matches!(output_mode, OutputMode::Json) => {
            "You are Shanka Magic Mode. Improve the user's selected text more boldly: make it polished, expressive, and contextually useful while preserving the user's intent and language. Return only valid json in this exact shape: {\"text\":\"rewritten text\"}. Do not include markdown, comments, or any extra keys.\n\nEXAMPLE INPUT:\n组织校内学术分享、职业规划讲座与跨院交流活动共15场\nEXAMPLE JSON OUTPUT:\n{\"text\":\"策划并组织校内学术分享、职业规划讲座及跨院交流活动 15 场，促进学生成长与院系协同。\"}"
        }
        SelectionMode::Safe => {
            "You are Shanka Safe Mode. Rewrite the user's selected text to be clear, natural, and grammatically correct while preserving the original meaning, tone, language, formatting, and approximate length. Return only the rewritten text."
        }
        SelectionMode::Magic => {
            "You are Shanka Magic Mode. Improve the user's selected text more boldly: make it polished, expressive, and contextually useful while preserving the user's intent and language. Return only the rewritten text."
        }
    };
    let instruction = match persona {
        Some(persona) => format!(
            "{base_instruction}\n\nTemporary persona: {}. {} Apply this persona for this rewrite only, while respecting the mode rules above.",
            persona.name, persona.system_prompt
        ),
        None => base_instruction.to_string(),
    };

    vec![
        ChatMessage {
            role: "system",
            content: instruction,
        },
        ChatMessage {
            role: "user",
            content: text.to_string(),
        },
    ]
}

impl OutputMode {
    fn label(self) -> &'static str {
        match self {
            Self::Json => "JSON",
            Self::Plain => "plain",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        config, parse_structured_rewrite, preferred_output_mode, OutputMode, ProviderCallError,
    };

    #[test]
    fn preferred_output_mode_skips_json_for_deepseek_provider() {
        let settings = config::AppSettingsConfig {
            provider: "deepseek".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            ..config::AppSettingsConfig::default()
        };

        assert_eq!(preferred_output_mode(&settings), OutputMode::Plain);
    }

    #[test]
    fn preferred_output_mode_skips_json_for_custom_deepseek_url() {
        let settings = config::AppSettingsConfig {
            provider: "custom".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            ..config::AppSettingsConfig::default()
        };

        assert_eq!(preferred_output_mode(&settings), OutputMode::Plain);
    }

    #[test]
    fn preferred_output_mode_uses_json_for_generic_openai_compatible_provider() {
        let settings = config::AppSettingsConfig {
            provider: "openai".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            ..config::AppSettingsConfig::default()
        };

        assert_eq!(preferred_output_mode(&settings), OutputMode::Json);
    }

    #[test]
    fn parse_structured_rewrite_accepts_clean_json_payload() {
        let output =
            parse_structured_rewrite(r#"{"text":"专业技能"}"#).expect("clean JSON should parse");

        assert_eq!(output, "专业技能");
    }

    #[test]
    fn parse_structured_rewrite_accepts_json_wrapped_in_text() {
        let output = parse_structured_rewrite(
            "```json\n{\"text\":\"组织校内学术分享、职业规划讲座与跨院交流活动共 15 场\"}\n```",
        )
        .expect("wrapped JSON should parse");

        assert_eq!(
            output,
            "组织校内学术分享、职业规划讲座与跨院交流活动共 15 场"
        );
    }

    #[test]
    fn parse_structured_rewrite_marks_empty_json_content_as_plain_retryable() {
        let error = parse_structured_rewrite("").expect_err("empty content should fail");

        assert_provider_error_flags(error, true, false, true);
    }

    fn assert_provider_error_flags(
        error: ProviderCallError,
        retry_plain: bool,
        retry_same: bool,
        invalid_response: bool,
    ) {
        assert_eq!(error.retry_plain, retry_plain);
        assert_eq!(error.retry_same, retry_same);
        assert_eq!(error.invalid_response, invalid_response);
    }
}
