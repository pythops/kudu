use std::{net::Ipv4Addr, str::FromStr};

use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::ListItem,
};
use serde::{Deserialize, Serialize};
use tui_input::{Input, backend::crossterm::EventHandler};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VNC {
    pub host: Ipv4Addr,
    pub password: Option<String>,
}

impl VNC {
    pub fn to_qemu_arg(&self) -> Vec<String> {
        let mut arg = format!("{}:0,to=99", self.host);

        if self.password.is_some() {
            arg.push_str(",password=on");
        }

        vec!["-vnc".to_string(), arg]
    }
}

// Builder

#[derive(Debug, Default, Clone, PartialEq)]
pub enum Section {
    #[default]
    Enable,
    Host,
    Password,
}

#[derive(Debug, Clone, Default)]
pub struct UserInputField {
    field: Input,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VncBuilder {
    pub section: Section,
    pub enabled: bool,
    pub host: UserInputField,
    pub password: UserInputField,
}

impl Default for VncBuilder {
    fn default() -> Self {
        Self {
            section: Section::Enable,
            enabled: true,
            host: UserInputField {
                field: Input::from("127.0.0.1"),
                error: None,
            },
            password: UserInputField::default(),
        }
    }
}

impl VncBuilder {
    pub fn new(enabled: bool, host: Ipv4Addr, password: Option<String>) -> Self {
        VncBuilder {
            section: Section::Enable,
            enabled,
            host: UserInputField {
                field: Input::from(host.to_string()),
                error: None,
            },
            password: UserInputField {
                field: Input::from(password.unwrap_or_default()),
                error: None,
            },
        }
    }
    pub fn build(&self) -> Option<VNC> {
        if self.enabled {
            Some(VNC {
                host: self.host(),
                password: self.passowrd(),
            })
        } else {
            None
        }
    }

    pub fn passowrd(&self) -> Option<String> {
        let value = self.password.field.value().to_string();
        if !value.is_empty() { Some(value) } else { None }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn host(&self) -> Ipv4Addr {
        Ipv4Addr::from_str(self.host.field.value()).unwrap()
    }

    fn is_authentication_enabled(&self) -> bool {
        !self.password.field.value().is_empty()
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        match self.section {
            Section::Enable => match key_event.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    self.section = Section::Host;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.section = Section::Password;
                }
                KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l') => {
                    self.enabled = !self.enabled;
                }
                _ => {}
            },
            Section::Host => match key_event.code {
                KeyCode::Down => {
                    self.section = Section::Password;
                }
                KeyCode::Up => {
                    self.section = Section::Enable;
                }
                _ => {
                    self.host
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
            },
            Section::Password => match key_event.code {
                KeyCode::Down => {
                    self.section = Section::Enable;
                }
                KeyCode::Up => {
                    self.section = Section::Host;
                }
                _ => {
                    self.password
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
            },
        }
    }

    pub fn validate(&mut self) -> bool {
        let mut valid = true;

        self.host.error = None;
        self.password.error = None;

        if !self.host.field.value().is_empty()
            && Ipv4Addr::from_str(self.host.field.value()).is_err()
        {
            self.host.error = Some("Invalid host addresse".into());
            valid = false;
        }

        if self.password.field.value().len() > 8 {
            self.password.error = Some("The password is limited to 8 characters".into());
            valid = false;
        }

        valid
    }

    pub fn summary(&self) -> Vec<ListItem<'_>> {
        vec![ListItem::from(vec![
            Line::from(vec![
                Span::from("Remote Access").bold(),
                Span::from(" ".repeat(7)),
                Span::from({
                    if self.enabled {
                        format!(
                            "VNC: host={} , port=auto , authentication={}",
                            self.host(),
                            self.is_authentication_enabled()
                        )
                    } else {
                        "Disabled".to_string()
                    }
                }),
            ]),
            Line::from(""),
        ])]
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, cancel_confirmation_popup: bool) {
        let (title_block, enabled_block, host_block, password_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Length(4),
                ])
                .margin(2)
                .split(area);

            (chunks[0], chunks[1], chunks[2], chunks[3])
        };

        let title = Text::from("VNC").bold().centered();

        let enabled = Line::from(vec![
            {
                if self.section == Section::Enable {
                    Span::from("> Status").bold()
                } else {
                    Span::from("  Status")
                }
            },
            Span::from(" ".repeat(6)),
            Span::from({
                if self.enabled {
                    "[x] Enabled                [ ] Disabled"
                } else {
                    "[ ] Enabled                [x] Disabled"
                }
            }),
        ]);

        let host = vec![
            Line::from(vec![
                {
                    if self.section == Section::Host {
                        Span::from("> Host   ").bold()
                    } else {
                        Span::from("  Host   ")
                    }
                },
                Span::from(" ".repeat(5)),
                Span::from({
                    let original_length = self.host.field.to_string().len();
                    let target_length = 65_usize;

                    self.host
                        .field
                        .to_string()
                        .chars()
                        .chain(std::iter::repeat_n(
                            ' ',
                            target_length.saturating_sub(original_length),
                        ))
                        .collect::<String>()
                })
                .on_dark_gray(),
            ]),
            Line::from(vec![
                Span::from(" ".repeat(14)),
                Span::from(self.host.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];

        let password = vec![
            Line::from(vec![
                {
                    if self.section == Section::Password {
                        Span::from("> Password").bold()
                    } else {
                        Span::from("  Password")
                    }
                },
                Span::from(" ".repeat(4)),
                Span::from({
                    let original_length = self.password.field.to_string().len();
                    let target_length = 65_usize;

                    self.password
                        .field
                        .to_string()
                        .chars()
                        .chain(std::iter::repeat_n(
                            ' ',
                            target_length.saturating_sub(original_length),
                        ))
                        .collect::<String>()
                })
                .on_dark_gray(),
            ]),
            Line::from(vec![
                Span::from(" ".repeat(14)),
                Span::from(self.password.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];

        frame.render_widget(title, title_block);
        frame.render_widget(Text::from(enabled), enabled_block);
        if self.enabled {
            frame.render_widget(Text::from(host), host_block);
            frame.render_widget(Text::from(password), password_block);

            if !cancel_confirmation_popup {
                match self.section {
                    Section::Host if self.host.field.visual_cursor() < 50 => {
                        let x = area.x + self.host.field.visual_cursor() as u16 + 16;
                        let y = area.y + 10;
                        frame.set_cursor_position((x, y));
                    }
                    Section::Password if self.password.field.visual_cursor() <= 50 => {
                        let x = area.x + self.password.field.visual_cursor() as u16 + 16;
                        let y = area.y + 14;
                        frame.set_cursor_position((x, y));
                    }
                    _ => {}
                }
            }
        }
    }
}
