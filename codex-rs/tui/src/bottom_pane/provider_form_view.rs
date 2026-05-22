use codex_app_server_protocol::ProviderAuthStyle;
use codex_app_server_protocol::ProviderConfigParams;
use codex_app_server_protocol::ProviderCreateParams;
use codex_app_server_protocol::ProviderUpdateParams;
use codex_app_server_protocol::ProviderWireApi;
use codex_model_provider_info::AuthStyle as ModelProviderAuthStyle;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::WireApi as ModelProviderWireApi;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use crate::render::renderable::Renderable;

use super::CancellationEvent;
use super::bottom_pane_view::BottomPaneView;
use super::bottom_pane_view::ViewCompletion;

const ROW_LABEL_WIDTH: usize = 18;
const CREATE_REQUEST_MAX_RETRIES: &str = "4";
const CREATE_STREAM_MAX_RETRIES: &str = "5";
const CREATE_STREAM_IDLE_TIMEOUT_MS: &str = "300000";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProviderFormMode {
    Create,
    Edit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProviderFormRow {
    DisplayName,
    BaseUrl,
    WireApi,
    AuthStyle,
    ApiKey,
    EnvKey,
    Save,
}

const CREATE_ROWS: [ProviderFormRow; 7] = [
    ProviderFormRow::DisplayName,
    ProviderFormRow::BaseUrl,
    ProviderFormRow::ApiKey,
    ProviderFormRow::EnvKey,
    ProviderFormRow::WireApi,
    ProviderFormRow::AuthStyle,
    ProviderFormRow::Save,
];

const EDIT_ROWS: [ProviderFormRow; 7] = [
    ProviderFormRow::DisplayName,
    ProviderFormRow::BaseUrl,
    ProviderFormRow::ApiKey,
    ProviderFormRow::EnvKey,
    ProviderFormRow::WireApi,
    ProviderFormRow::AuthStyle,
    ProviderFormRow::Save,
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct TextField {
    value: String,
    cursor: usize,
    placeholder: &'static str,
    secret: bool,
}

impl TextField {
    fn new(value: impl Into<String>, placeholder: &'static str, secret: bool) -> Self {
        let value = value.into();
        let cursor = value.chars().count();
        Self {
            value,
            cursor,
            placeholder,
            secret,
        }
    }

    fn display_text(&self) -> String {
        if self.secret && !self.value.is_empty() {
            "*".repeat(self.value.chars().count())
        } else {
            self.value.clone()
        }
    }

    fn display_width_to_cursor(&self) -> usize {
        let visible_text = if self.secret && !self.value.is_empty() {
            "*".repeat(self.cursor)
        } else {
            self.value.chars().take(self.cursor).collect::<String>()
        };
        UnicodeWidthStr::width(visible_text.as_str())
    }

    fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_right(&mut self) {
        let len = self.value.chars().count();
        if self.cursor < len {
            self.cursor += 1;
        }
    }

    fn move_home(&mut self) {
        self.cursor = 0;
    }

    fn move_end(&mut self) {
        self.cursor = self.value.chars().count();
    }

    fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    fn insert_char(&mut self, ch: char) {
        let mut chars = self.value.chars().collect::<Vec<_>>();
        chars.insert(self.cursor, ch);
        self.value = chars.into_iter().collect();
        self.cursor += 1;
    }

    fn insert_str(&mut self, text: &str) {
        for ch in text.chars() {
            self.insert_char(ch);
        }
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut chars = self.value.chars().collect::<Vec<_>>();
        chars.remove(self.cursor - 1);
        self.value = chars.into_iter().collect();
        self.cursor -= 1;
    }

    fn delete(&mut self) {
        let mut chars = self.value.chars().collect::<Vec<_>>();
        if self.cursor >= chars.len() {
            return;
        }
        chars.remove(self.cursor);
        self.value = chars.into_iter().collect();
    }

    fn trimmed_value(&self) -> String {
        self.value.trim().to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ProviderFormSubmission {
    Create(ProviderCreateParams),
    Update(ProviderUpdateParams),
}

pub(crate) type ProviderFormSubmitted = Box<dyn Fn(ProviderFormSubmission) + Send + Sync>;

pub(crate) struct ProviderFormView {
    mode: ProviderFormMode,
    provider_id: Option<String>,
    active_row: usize,
    completion: Option<ViewCompletion>,
    error_message: Option<String>,
    on_submit: ProviderFormSubmitted,
    display_name_field: TextField,
    base_url_field: TextField,
    api_key_field: TextField,
    env_key_field: TextField,
    request_max_retries_field: TextField,
    stream_max_retries_field: TextField,
    stream_idle_timeout_ms_field: TextField,
    wire_api: ProviderWireApi,
    auth_style: ProviderAuthStyle,
    supports_websockets: bool,
}

impl ProviderFormView {
    pub(crate) fn new_create(on_submit: ProviderFormSubmitted) -> Self {
        Self {
            mode: ProviderFormMode::Create,
            provider_id: None,
            active_row: 0,
            completion: None,
            error_message: None,
            on_submit,
            display_name_field: TextField::new("", "OpenRouter Custom", /*secret*/ false),
            base_url_field: TextField::new(
                "",
                "https://openrouter.ai/api/v1",
                /*secret*/ false,
            ),
            api_key_field: TextField::new("", "sk-...", /*secret*/ true),
            env_key_field: TextField::new("", "OPENROUTER_API_KEY", /*secret*/ false),
            request_max_retries_field: TextField::new(
                CREATE_REQUEST_MAX_RETRIES,
                "",
                /*secret*/ false,
            ),
            stream_max_retries_field: TextField::new(
                CREATE_STREAM_MAX_RETRIES,
                "",
                /*secret*/ false,
            ),
            stream_idle_timeout_ms_field: TextField::new(
                CREATE_STREAM_IDLE_TIMEOUT_MS,
                "",
                /*secret*/ false,
            ),
            wire_api: ProviderWireApi::Responses,
            auth_style: ProviderAuthStyle::Bearer,
            supports_websockets: false,
        }
    }

    pub(crate) fn new_edit(
        provider_id: String,
        provider: &ModelProviderInfo,
        on_submit: ProviderFormSubmitted,
    ) -> Self {
        Self {
            mode: ProviderFormMode::Edit,
            provider_id: Some(provider_id),
            active_row: 0,
            completion: None,
            error_message: None,
            on_submit,
            display_name_field: TextField::new(provider.name.clone(), "", /*secret*/ false),
            base_url_field: TextField::new(
                provider.base_url.clone().unwrap_or_default(),
                "",
                /*secret*/ false,
            ),
            api_key_field: TextField::new(
                "",
                "Leave blank to keep the saved key",
                /*secret*/ true,
            ),
            env_key_field: TextField::new(
                provider.env_key.clone().unwrap_or_default(),
                "OPENROUTER_API_KEY",
                /*secret*/ false,
            ),
            request_max_retries_field: TextField::new(
                provider
                    .request_max_retries
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                "",
                /*secret*/ false,
            ),
            stream_max_retries_field: TextField::new(
                provider
                    .stream_max_retries
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                "",
                /*secret*/ false,
            ),
            stream_idle_timeout_ms_field: TextField::new(
                provider
                    .stream_idle_timeout_ms
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                "",
                /*secret*/ false,
            ),
            wire_api: match provider.wire_api {
                ModelProviderWireApi::Responses => ProviderWireApi::Responses,
                ModelProviderWireApi::AnthropicMessages => ProviderWireApi::AnthropicMessages,
                ModelProviderWireApi::ChatCompletions => ProviderWireApi::ChatCompletions,
            },
            auth_style: match provider.auth_style {
                ModelProviderAuthStyle::Bearer => ProviderAuthStyle::Bearer,
                ModelProviderAuthStyle::XApiKey => ProviderAuthStyle::XApiKey,
            },
            supports_websockets: provider.supports_websockets,
        }
    }

    fn rows(&self) -> &'static [ProviderFormRow] {
        match self.mode {
            ProviderFormMode::Create => &CREATE_ROWS,
            ProviderFormMode::Edit => &EDIT_ROWS,
        }
    }

    fn active_row_kind(&self) -> ProviderFormRow {
        self.rows()[self.active_row]
    }

    fn title(&self) -> &'static str {
        match self.mode {
            ProviderFormMode::Create => "Create provider",
            ProviderFormMode::Edit => "Edit provider",
        }
    }

    fn context_line(&self) -> Option<String> {
        self.provider_id
            .as_ref()
            .map(|provider_id| format!("Editing `{provider_id}`"))
    }

    fn move_prev_row(&mut self) {
        if self.active_row == 0 {
            self.active_row = self.rows().len().saturating_sub(1);
        } else {
            self.active_row -= 1;
        }
        self.error_message = None;
    }

    fn move_next_row(&mut self) {
        self.active_row = (self.active_row + 1) % self.rows().len();
        self.error_message = None;
    }

    fn cycle_wire_api_forward(&mut self) {
        self.wire_api = match self.wire_api {
            ProviderWireApi::Responses => ProviderWireApi::AnthropicMessages,
            ProviderWireApi::AnthropicMessages => ProviderWireApi::ChatCompletions,
            ProviderWireApi::ChatCompletions => ProviderWireApi::Responses,
        };
        self.error_message = None;
    }

    fn cycle_wire_api_backward(&mut self) {
        self.wire_api = match self.wire_api {
            ProviderWireApi::Responses => ProviderWireApi::ChatCompletions,
            ProviderWireApi::AnthropicMessages => ProviderWireApi::Responses,
            ProviderWireApi::ChatCompletions => ProviderWireApi::AnthropicMessages,
        };
        self.error_message = None;
    }

    fn cycle_auth_style_forward(&mut self) {
        self.auth_style = match self.auth_style {
            ProviderAuthStyle::Bearer => ProviderAuthStyle::XApiKey,
            ProviderAuthStyle::XApiKey => ProviderAuthStyle::Bearer,
        };
        self.error_message = None;
    }

    fn cycle_auth_style_backward(&mut self) {
        self.cycle_auth_style_forward();
    }

    fn active_text_field_mut(&mut self) -> Option<&mut TextField> {
        match self.active_row_kind() {
            ProviderFormRow::DisplayName => Some(&mut self.display_name_field),
            ProviderFormRow::BaseUrl => Some(&mut self.base_url_field),
            ProviderFormRow::ApiKey => Some(&mut self.api_key_field),
            ProviderFormRow::EnvKey => Some(&mut self.env_key_field),
            ProviderFormRow::WireApi | ProviderFormRow::AuthStyle | ProviderFormRow::Save => None,
        }
    }

    fn active_text_field(&self) -> Option<&TextField> {
        match self.active_row_kind() {
            ProviderFormRow::DisplayName => Some(&self.display_name_field),
            ProviderFormRow::BaseUrl => Some(&self.base_url_field),
            ProviderFormRow::ApiKey => Some(&self.api_key_field),
            ProviderFormRow::EnvKey => Some(&self.env_key_field),
            ProviderFormRow::WireApi | ProviderFormRow::AuthStyle | ProviderFormRow::Save => None,
        }
    }

    fn active_row_label(&self) -> &'static str {
        match self.active_row_kind() {
            ProviderFormRow::DisplayName => "Display name",
            ProviderFormRow::BaseUrl => "Base URL",
            ProviderFormRow::WireApi => "Wire API",
            ProviderFormRow::AuthStyle => "Auth style",
            ProviderFormRow::ApiKey => "API key",
            ProviderFormRow::EnvKey => "Env key",
            ProviderFormRow::Save => "Save provider",
        }
    }

    fn row_line(&self, row: ProviderFormRow, active: bool) -> Line<'static> {
        let marker = if active { "> ".cyan() } else { "  ".into() };
        match row {
            ProviderFormRow::Save => {
                let label = if active {
                    Span::from("Save provider").green().bold()
                } else {
                    Span::from("Save provider").green()
                };
                Line::from(vec![marker, label])
            }
            ProviderFormRow::WireApi => self.option_line(
                marker,
                self.label_span("Wire API", active),
                self.wire_api_label(),
                active,
            ),
            ProviderFormRow::AuthStyle => self.option_line(
                marker,
                self.label_span("Auth style", active),
                self.auth_style_label(),
                active,
            ),
            ProviderFormRow::DisplayName => self.text_line(
                marker,
                self.label_span("Display name", active),
                &self.display_name_field,
            ),
            ProviderFormRow::BaseUrl => self.text_line(
                marker,
                self.label_span("Base URL", active),
                &self.base_url_field,
            ),
            ProviderFormRow::ApiKey => self.text_line(
                marker,
                self.label_span("API key", active),
                &self.api_key_field,
            ),
            ProviderFormRow::EnvKey => self.text_line(
                marker,
                self.label_span("Env key", active),
                &self.env_key_field,
            ),
        }
    }

    fn text_line(
        &self,
        marker: Span<'static>,
        label: Span<'static>,
        field: &TextField,
    ) -> Line<'static> {
        let value = if field.value.is_empty() {
            Span::from(field.placeholder).dim()
        } else {
            Span::from(field.display_text())
        };
        Line::from(vec![marker, label, " [".into(), value, "]".dim()])
    }

    fn option_line(
        &self,
        marker: Span<'static>,
        label: Span<'static>,
        value: &'static str,
        active: bool,
    ) -> Line<'static> {
        let value = if active {
            Span::from(value).bold()
        } else {
            Span::from(value)
        };
        Line::from(vec![marker, label, " [".into(), value, "]".dim()])
    }

    fn label_span(&self, label: &'static str, active: bool) -> Span<'static> {
        let padded = format!("{label:ROW_LABEL_WIDTH$}");
        if active {
            Span::from(padded).cyan()
        } else {
            Span::from(padded).dim()
        }
    }

    fn wire_api_label(&self) -> &'static str {
        match self.wire_api {
            ProviderWireApi::Responses => "responses",
            ProviderWireApi::AnthropicMessages => "anthropic_messages",
            ProviderWireApi::ChatCompletions => "chat_completions",
        }
    }

    fn auth_style_label(&self) -> &'static str {
        match self.auth_style {
            ProviderAuthStyle::Bearer => "bearer",
            ProviderAuthStyle::XApiKey => "x_api_key",
        }
    }

    fn submit(&mut self) {
        match self.build_submission() {
            Ok(submission) => {
                (self.on_submit)(submission);
                self.completion = Some(ViewCompletion::Accepted);
            }
            Err(err) => {
                self.error_message = Some(err);
            }
        }
    }

    fn build_submission(&self) -> Result<ProviderFormSubmission, String> {
        let display_name = self.display_name_field.trimmed_value();
        let base_url = self.base_url_field.trimmed_value();
        let env_key = self.optional_trimmed(&self.env_key_field);
        let api_key = self.optional_trimmed(&self.api_key_field);

        if display_name.is_empty() {
            return Err("Display name is required.".to_string());
        }
        if base_url.is_empty() {
            return Err("Base URL is required.".to_string());
        }
        if api_key.is_some() && env_key.is_some() {
            return Err("Use either API key or Env key, not both.".to_string());
        }

        let generated_provider_id = Self::generated_provider_id(&display_name, &base_url);
        let provider = ProviderConfigParams {
            display_name,
            base_url,
            wire_api: self.wire_api,
            auth_style: self.auth_style,
            env_key,
            api_key,
            requires_openai_auth: false,
            supports_websockets: self.supports_websockets,
            request_max_retries: self
                .parse_optional_u64(&self.request_max_retries_field, "Request retries")?,
            stream_max_retries: self
                .parse_optional_u64(&self.stream_max_retries_field, "Stream retries")?,
            stream_idle_timeout_ms: self
                .parse_optional_u64(&self.stream_idle_timeout_ms_field, "Idle timeout ms")?,
            websocket_connect_timeout_ms: None,
            headers: None,
            env_headers: None,
        };

        Ok(match self.mode {
            ProviderFormMode::Create => ProviderFormSubmission::Create(ProviderCreateParams {
                id: generated_provider_id,
                provider,
                set_default: false,
                default_model: None,
            }),
            ProviderFormMode::Edit => ProviderFormSubmission::Update(ProviderUpdateParams {
                id: self.provider_id.clone().unwrap_or_default(),
                provider,
                set_default: false,
                default_model: None,
            }),
        })
    }

    fn optional_trimmed(&self, field: &TextField) -> Option<String> {
        let trimmed = field.trimmed_value();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    fn generated_provider_id(display_name: &str, base_url: &str) -> String {
        let host = base_url
            .split_once("://")
            .map_or(base_url, |(_, remainder)| remainder)
            .split('/')
            .next()
            .unwrap_or(base_url);
        let seed = if display_name.is_empty() {
            host
        } else {
            display_name
        };

        let mut slug = String::new();
        let mut last_was_dash = false;
        for ch in seed.chars() {
            if ch.is_ascii_alphanumeric() {
                slug.push(ch.to_ascii_lowercase());
                last_was_dash = false;
            } else if !last_was_dash && !slug.is_empty() {
                slug.push('-');
                last_was_dash = true;
            }
        }

        while slug.ends_with('-') {
            slug.pop();
        }

        if slug.is_empty() {
            "custom-provider".to_string()
        } else {
            slug
        }
    }

    fn parse_optional_u64(&self, field: &TextField, label: &str) -> Result<Option<u64>, String> {
        let trimmed = field.trimmed_value();
        if trimmed.is_empty() {
            return Ok(None);
        }
        trimmed
            .parse::<u64>()
            .map(Some)
            .map_err(|_| format!("{label} must be a non-negative integer."))
    }

    fn text_cursor_x(&self, area: Rect) -> Option<u16> {
        let field = self.active_text_field()?;
        let prefix = format!(
            "{:width$} [",
            self.active_row_label(),
            width = ROW_LABEL_WIDTH
        );
        let prefix_width = 2 + UnicodeWidthStr::width(prefix.as_str());
        let cursor_width = field.display_width_to_cursor();
        let max_x = area.x.saturating_add(area.width.saturating_sub(1));
        let x = area.x.saturating_add((prefix_width + cursor_width) as u16);
        Some(x.min(max_x))
    }
}

impl BottomPaneView for ProviderFormView {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event {
            KeyEvent {
                code: KeyCode::Esc, ..
            } => {
                self.on_ctrl_c();
            }
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
                ..
            } if modifiers.contains(KeyModifiers::CONTROL) && c.eq_ignore_ascii_case(&'s') => {
                self.submit();
            }
            KeyEvent {
                code: KeyCode::Up, ..
            }
            | KeyEvent {
                code: KeyCode::BackTab,
                ..
            } => self.move_prev_row(),
            KeyEvent {
                code: KeyCode::Down,
                ..
            }
            | KeyEvent {
                code: KeyCode::Tab, ..
            } => self.move_next_row(),
            KeyEvent {
                code: KeyCode::Left,
                ..
            } => match self.active_row_kind() {
                ProviderFormRow::WireApi => self.cycle_wire_api_backward(),
                ProviderFormRow::AuthStyle => self.cycle_auth_style_backward(),
                _ => {
                    if let Some(field) = self.active_text_field_mut() {
                        field.move_left();
                    }
                }
            },
            KeyEvent {
                code: KeyCode::Right,
                ..
            } => match self.active_row_kind() {
                ProviderFormRow::WireApi => self.cycle_wire_api_forward(),
                ProviderFormRow::AuthStyle => self.cycle_auth_style_forward(),
                _ => {
                    if let Some(field) = self.active_text_field_mut() {
                        field.move_right();
                    }
                }
            },
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => match self.active_row_kind() {
                ProviderFormRow::WireApi => self.cycle_wire_api_forward(),
                ProviderFormRow::AuthStyle => self.cycle_auth_style_forward(),
                ProviderFormRow::Save => self.submit(),
                _ => self.move_next_row(),
            },
            KeyEvent {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::NONE,
                ..
            } => match self.active_row_kind() {
                ProviderFormRow::WireApi => self.cycle_wire_api_forward(),
                ProviderFormRow::AuthStyle => self.cycle_auth_style_forward(),
                _ => {
                    if let Some(field) = self.active_text_field_mut() {
                        field.insert_char(' ');
                        self.error_message = None;
                    }
                }
            },
            KeyEvent {
                code: KeyCode::Home,
                ..
            }
            | KeyEvent {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                if let Some(field) = self.active_text_field_mut() {
                    field.move_home();
                }
            }
            KeyEvent {
                code: KeyCode::End, ..
            }
            | KeyEvent {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                if let Some(field) = self.active_text_field_mut() {
                    field.move_end();
                }
            }
            KeyEvent {
                code: KeyCode::Backspace,
                ..
            } => {
                if let Some(field) = self.active_text_field_mut() {
                    field.backspace();
                    self.error_message = None;
                }
            }
            KeyEvent {
                code: KeyCode::Delete,
                ..
            } => {
                if let Some(field) = self.active_text_field_mut() {
                    field.delete();
                    self.error_message = None;
                }
            }
            KeyEvent {
                code: KeyCode::Char('u'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                if let Some(field) = self.active_text_field_mut() {
                    field.clear();
                    self.error_message = None;
                }
            }
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
                ..
            } if !modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER) =>
            {
                if let Some(field) = self.active_text_field_mut() {
                    field.insert_char(c);
                    self.error_message = None;
                }
            }
            _ => {}
        }
    }

    fn on_ctrl_c(&mut self) -> CancellationEvent {
        self.completion = Some(ViewCompletion::Cancelled);
        CancellationEvent::Handled
    }

    fn is_complete(&self) -> bool {
        self.completion.is_some()
    }

    fn completion(&self) -> Option<ViewCompletion> {
        self.completion
    }

    fn handle_paste(&mut self, pasted: String) -> bool {
        let Some(field) = self.active_text_field_mut() else {
            return false;
        };
        let sanitized = pasted
            .chars()
            .map(|ch| if matches!(ch, '\n' | '\r') { ' ' } else { ch })
            .collect::<String>();
        if sanitized.is_empty() {
            return false;
        }
        field.insert_str(&sanitized);
        self.error_message = None;
        true
    }
}

impl Renderable for ProviderFormView {
    fn desired_height(&self, _width: u16) -> u16 {
        let title_height = 1u16;
        let context_height: u16 = if self.context_line().is_some() { 1 } else { 0 };
        let error_height: u16 = if self.error_message.is_some() { 1 } else { 0 };
        title_height + context_height + self.rows().len() as u16 + error_height + 1
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        Clear.render(area, buf);

        let title_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        Paragraph::new(Line::from(self.title().bold())).render(title_area, buf);

        let mut y = area.y.saturating_add(1);
        if let Some(context) = self.context_line() {
            Paragraph::new(Line::from(context.dim())).render(
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
            y = y.saturating_add(1);
        }

        for (index, row) in self.rows().iter().copied().enumerate() {
            Paragraph::new(self.row_line(row, index == self.active_row)).render(
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
            y = y.saturating_add(1);
        }

        if let Some(error) = &self.error_message {
            Paragraph::new(Line::from(error.clone().red())).render(
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
            y = y.saturating_add(1);
        }

        Paragraph::new(Line::from(
            "tab move | enter next/change | ctrl+s save | esc cancel".dim(),
        ))
        .render(
            Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            },
            buf,
        );
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.active_text_field()?;
        let context_offset: u16 = if self.context_line().is_some() { 1 } else { 0 };
        let y = area
            .y
            .saturating_add(1)
            .saturating_add(context_offset)
            .saturating_add(self.active_row as u16);
        Some((self.text_cursor_x(area)?, y))
    }
}
