use crossterm::event::{KeyCode, KeyEvent};
use serde::{Deserialize, Serialize};
use tui_input::{Input, backend::crossterm::EventHandler};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear},
};

#[derive(Debug, Clone, Copy, strum::Display, Deserialize, Serialize)]
pub enum NetworkBackend {
    #[strum(to_string = "User - {0}")]
    User(UserMode),
}

impl NetworkBackend {
    pub fn to_qemu_arg(&self, port_mappings: &[PortMapping]) -> Vec<String> {
        let mapping_arg = port_mappings
            .iter()
            .map(|mapping| {
                format!(
                    "hostfwd={}::{}-:{}",
                    mapping.protocol.to_string().to_lowercase(),
                    mapping.host_port,
                    mapping.guest_port
                )
            })
            .collect::<Vec<String>>()
            .join(",");

        let mut args = vec![
            "-device".to_string(),
            "virtio-net-pci,netdev=net0".to_string(),
            "-netdev".to_string(),
        ];

        if mapping_arg.is_empty() {
            args.push("user,id=net0".to_string());
        } else {
            args.push(format!("user,id=net0,{}", mapping_arg));
        }

        args
    }
}

impl Default for NetworkBackend {
    fn default() -> Self {
        NetworkBackend::User(UserMode::Slirp)
    }
}

#[derive(Debug, Default, Clone, Copy, strum::Display, Deserialize, Serialize)]
#[strum(serialize_all = "UPPERCASE")]
pub enum UserMode {
    #[default]
    Slirp,
    Passt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortMapping {
    pub guest_port: u16,
    pub host_port: u16,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, strum::Display, Serialize, Deserialize)]
pub enum Protocol {
    #[default]
    TCP,
    UDP,
    // UNIX //TODO:
}

// Port Forwarding Builder

#[derive(Debug, Clone, Default)]
struct UserInputField {
    field: Input,
    error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
enum Section {
    #[default]
    GuestPort,
    HostPort,
    Protocol,
}

#[derive(Debug, Clone, Default)]
pub struct MappingBuilder {
    section: Section,
    guest_port: UserInputField,
    host_port: UserInputField,
    protocol: Protocol,
}

impl MappingBuilder {
    pub fn new() -> Self {
        MappingBuilder::default()
    }
    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Down | KeyCode::Char('j') => match self.section {
                Section::GuestPort => self.section = Section::HostPort,
                Section::HostPort => self.section = Section::Protocol,
                Section::Protocol => self.section = Section::GuestPort,
            },
            KeyCode::Up | KeyCode::Char('k') => match self.section {
                Section::GuestPort => self.section = Section::Protocol,
                Section::HostPort => self.section = Section::GuestPort,
                Section::Protocol => self.section = Section::HostPort,
            },

            KeyCode::Right | KeyCode::Char('l') if self.section == Section::Protocol => {
                match self.protocol {
                    Protocol::TCP => self.protocol = Protocol::UDP,
                    Protocol::UDP => self.protocol = Protocol::TCP,
                }
            }
            KeyCode::Left | KeyCode::Char('h') if self.section == Section::Protocol => {
                match self.protocol {
                    Protocol::TCP => self.protocol = Protocol::UDP,
                    Protocol::UDP => self.protocol = Protocol::TCP,
                }
            }

            _ => match self.section {
                Section::GuestPort => {
                    self.guest_port
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
                Section::HostPort => {
                    self.host_port
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
                _ => {}
            },
        }
    }

    pub fn validate(&mut self) -> bool {
        let mut valid = true;

        self.guest_port.error = None;
        self.host_port.error = None;

        if self.host_port.field.value().is_empty() {
            self.host_port.error = Some("Field required".into());
            valid = false;
        } else {
            match self.host_port.field.value().parse::<u16>() {
                Ok(v) => {
                    if v == 0 {
                        self.host_port.error = Some("Host Port can not be 0".into());
                        valid = false;
                    }
                }
                Err(_) => {
                    self.host_port.error = Some("Host Port value should be a number".into());
                    valid = false;
                }
            }
        }

        if self.guest_port.field.value().is_empty() {
            self.guest_port.error = Some("Field required".into());
            valid = false;
        } else {
            match self.guest_port.field.value().parse::<u16>() {
                Ok(v) => {
                    if v == 0 {
                        self.guest_port.error = Some("Guest Port can not be 0".into());
                        valid = false;
                    }
                }
                Err(_) => {
                    self.guest_port.error = Some("Guest Port value should be a number".into());
                    valid = false;
                }
            }
        }

        valid
    }

    pub fn build(&self) -> PortMapping {
        let guest_port = self.guest_port.field.value().parse::<u16>().unwrap();
        let host_port = self.host_port.field.value().parse::<u16>().unwrap();
        let protocol = self.protocol;

        PortMapping {
            guest_port,
            host_port,
            protocol,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(15),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(frame.area())[1];

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(60),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(area)[1];

        frame.render_widget(Clear, area);

        frame.render_widget(
            Block::new()
                .borders(Borders::all())
                .title(" New Port Forwaring   ")
                .border_type(BorderType::Thick)
                .title_alignment(ratatui::layout::HorizontalAlignment::Center)
                .border_style(Style::default().yellow()),
            area,
        );

        let area = area.inner(Margin {
            horizontal: 2,
            vertical: 2,
        });

        let (guest_port_block, host_port_block, protocol_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .margin(1)
                .split(area);

            (chunks[0], chunks[1], chunks[2])
        };

        let guest_port = vec![
            Line::from(vec![
                {
                    if self.section == Section::GuestPort {
                        Span::from("> Guest Port ").bold()
                    } else {
                        Span::from("  Guest Port ")
                    }
                },
                Span::from(" ".repeat(8)),
                Span::from({
                    let original_length = self.guest_port.field.to_string().len();
                    let target_length = 30_usize;

                    self.guest_port
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
                Span::from(" ".repeat(21)),
                Span::from(self.guest_port.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];

        let host_port = vec![
            Line::from(vec![
                {
                    if self.section == Section::HostPort {
                        Span::from("> Host Port  ").bold()
                    } else {
                        Span::from("  Host Port  ")
                    }
                },
                Span::from(" ".repeat(8)),
                Span::from({
                    let original_length = self.host_port.field.to_string().len();
                    let target_length = 30_usize;

                    self.host_port
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
                Span::from(" ".repeat(21)),
                Span::from(self.host_port.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];

        let protocol = Line::from(vec![
            {
                if self.section == Section::Protocol {
                    Span::from("> Protocol ").bold()
                } else {
                    Span::from("  Protocol ")
                }
            },
            Span::from(" ".repeat(10)),
            Span::from(format!("< {} >", self.protocol)),
        ]);

        frame.render_widget(Text::from(guest_port), guest_port_block);
        frame.render_widget(Text::from(host_port), host_port_block);
        frame.render_widget(Text::from(protocol), protocol_block);

        match self.section {
            Section::GuestPort if self.guest_port.field.visual_cursor() < 65 => {
                let x = area.x + self.guest_port.field.visual_cursor() as u16 + 22;
                let y = area.y + 1;
                frame.set_cursor_position((x, y));
            }
            Section::HostPort if self.host_port.field.visual_cursor() < 50 => {
                let x = area.x + self.host_port.field.visual_cursor() as u16 + 22;
                let y = area.y + 4;
                frame.set_cursor_position((x, y));
            }
            _ => {}
        }
    }
}
