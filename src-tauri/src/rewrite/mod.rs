use crate::{config, selection::SelectionMode};
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
    pub persona: Option<RewritePersona>,
}

#[derive(Debug, Clone, Copy)]
pub struct RewritePersona {
    pub id: &'static str,
    pub name: &'static str,
    pub system_prompt: &'static str,
}

const DEFAULT_SAFE_PERSONA_ID: &str = "clean-correction";
const PERSONAS: [RewritePersona; 3] = [
    RewritePersona {
        id: "workplace-eq",
        name: "High-EQ Workplace",
        system_prompt: "Rewrite plain or emotional wording into tactful workplace communication.",
    },
    RewritePersona {
        id: "academic-concise",
        name: "Academic Concise",
        system_prompt: "Remove colloquial wording, compress length, and keep academic rigor.",
    },
    RewritePersona {
        id: "clean-correction",
        name: "Clean Correction",
        system_prompt:
            "Correct typos, punctuation, and formatting without changing the author's voice.",
    },
];

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
    Remote(String),
}

impl std::fmt::Display for RewriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(message) => write!(formatter, "rewrite config error: {message}"),
            Self::EmptyInput => write!(formatter, "rewrite input is empty"),
            Self::Timeout => write!(formatter, "remote rewrite error: request timed out"),
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

#[derive(Clone, Copy)]
enum OutputMode {
    Json,
    Plain,
}

#[derive(Debug)]
struct ProviderCallError {
    message: String,
    retry_plain: bool,
    timeout: bool,
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
    let settings = config::load_or_create(app)
        .map_err(|error| RewriteError::Config(error.to_string()))?
        .settings
        .normalized()
        .map_err(|error| RewriteError::Config(error.to_string()))?;
    let persona = persona_id.map(resolve_persona);

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
    DEFAULT_SAFE_PERSONA_ID
}

fn resolve_persona(persona_id: &str) -> RewritePersona {
    PERSONAS
        .iter()
        .copied()
        .find(|persona| persona.id == persona_id)
        .unwrap_or_else(default_safe_persona)
}

fn default_safe_persona() -> RewritePersona {
    resolve_persona(DEFAULT_SAFE_PERSONA_ID)
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
    persona: Option<RewritePersona>,
    text: &str,
) -> Result<String, RewriteError> {
    match call_chat_completion(settings, mode, persona, text, OutputMode::Json).await {
        Ok(output) => Ok(output),
        Err(error) if error.timeout => Err(RewriteError::Timeout),
        Err(error) if error.retry_plain => {
            println!(
                "[rewrite] JSON mode failed; retrying plain content mode: {}",
                error.message
            );
            call_chat_completion(settings, mode, persona, text, OutputMode::Plain)
                .await
                .map_err(ProviderCallError::into_rewrite_error)
        }
        Err(error) => Err(error.into_rewrite_error()),
    }
}

async fn call_chat_completion(
    settings: &config::AppSettingsConfig,
    mode: SelectionMode,
    persona: Option<RewritePersona>,
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
                ProviderCallError::remote(format!("request failed: {error}"))
            }
        })?;

    let status = response.status();
    let body = response.bytes().await.map_err(|error| {
        ProviderCallError::remote(format!("failed to read provider response body: {error}"))
    })?;

    if !status.is_success() {
        let body = response_body_sample(&body);
        return Err(ProviderCallError {
            message: format!("provider returned HTTP {status}: {body}"),
            retry_plain: matches!(output_mode, OutputMode::Json)
                && response_format_may_be_unsupported(status.as_u16(), &body),
            timeout: false,
        });
    }

    let payload = serde_json::from_slice::<ChatCompletionResponse>(&body).map_err(|error| {
        ProviderCallError::remote(format!(
            "failed to parse provider response JSON: {error}; body: {}",
            response_body_sample(&body)
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
            timeout: false,
        });
    }

    Ok(output)
}

impl ProviderCallError {
    fn timeout() -> Self {
        Self {
            message: "request timed out".to_string(),
            retry_plain: false,
            timeout: true,
        }
    }

    fn remote(message: String) -> Self {
        Self {
            message,
            retry_plain: false,
            timeout: false,
        }
    }

    fn into_rewrite_error(self) -> RewriteError {
        if self.timeout {
            RewriteError::Timeout
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
            timeout: false,
        });
    }

    serde_json::from_str::<StructuredRewriteContent>(content)
        .map(|payload| payload.text.trim().to_string())
        .map_err(|error| ProviderCallError {
            message: format!(
                "failed to parse rewrite JSON content: {error}; content: {}",
                content.chars().take(500).collect::<String>()
            ),
            retry_plain: true,
            timeout: false,
        })
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

fn max_tokens_for_text(text: &str) -> u32 {
    let approximate_input_tokens = text.chars().count().saturating_div(2) as u32;
    approximate_input_tokens.clamp(256, 4_096)
}

fn response_body_sample(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let sample = text.trim().chars().take(500).collect::<String>();

    if sample.is_empty() {
        "<empty body>".to_string()
    } else {
        sample
    }
}

fn rewrite_messages(
    mode: SelectionMode,
    persona: Option<RewritePersona>,
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
