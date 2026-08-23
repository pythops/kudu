use std::{cell::RefCell, rc::Rc};

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

use crate::network::{Network, NetworkId};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    Network,
    GuestPort,
    HostPort,
    Protocol,
}

#[derive(Debug, Clone)]
pub struct MappingBuilder {
    section: Section,
    guest_port: UserInputField,
    host_port: UserInputField,
    protocol: Protocol,
    networks: Rc<RefCell<Vec<Network>>>,
    selected_network: usize,
}

impl MappingBuilder {
    pub fn new(networks: Rc<RefCell<Vec<Network>>>) -> Self {
        MappingBuilder {
            section: Section::default(),
            guest_port: UserInputField::default(),
            host_port: UserInputField::default(),
            protocol: Protocol::default(),
            networks,
            selected_network: 0,
        }
    }
    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Down | KeyCode::Char('j') => match self.section {
                Section::Network => self.section = Section::GuestPort,
                Section::GuestPort => self.section = Section::HostPort,
                Section::HostPort => self.section = Section::Protocol,
                Section::Protocol => self.section = Section::Network,
            },
            KeyCode::Up | KeyCode::Char('k') => match self.section {
                Section::Network => self.section = Section::Protocol,
                Section::GuestPort => self.section = Section::Network,
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

            KeyCode::Right | KeyCode::Char('l') if self.section == Section::Network => {
                let networks = self.networks.borrow();
                self.selected_network = if self.selected_network == networks.len() - 1 {
                    0
                } else {
                    self.selected_network.saturating_add(1)
                }
            }

            KeyCode::Left | KeyCode::Char('h') if self.section == Section::Network => {
                let networks = self.networks.borrow();
                self.selected_network = if self.selected_network == 0 {
                    networks.len() - 1
                } else {
                    self.selected_network.saturating_sub(1)
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

    pub fn build(&self) -> (NetworkId, PortMapping) {
        let guest_port = self.guest_port.field.value().parse::<u16>().unwrap();
        let host_port = self.host_port.field.value().parse::<u16>().unwrap();
        let protocol = self.protocol;

        let mapping = PortMapping {
            guest_port,
            host_port,
            protocol,
        };

        let network = &self.networks.borrow()[self.selected_network];

        (network.id.clone(), mapping)
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(18),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(frame.area())[1];

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(70),
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

        let (network_block, guest_port_block, host_port_block, protocol_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .margin(1)
                .split(area);

            (chunks[0], chunks[1], chunks[2], chunks[3])
        };

        let network = Line::from(vec![
            {
                if self.section == Section::Network {
                    Span::from("> Network  ").bold()
                } else {
                    Span::from("  Network  ")
                }
            },
            Span::from(" ".repeat(10)),
            Span::from(format!(
                "< {} >",
                self.networks.borrow()[self.selected_network].id
            )),
        ]);

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

        frame.render_widget(Text::from(network), network_block);
        frame.render_widget(Text::from(guest_port), guest_port_block);
        frame.render_widget(Text::from(host_port), host_port_block);
        frame.render_widget(Text::from(protocol), protocol_block);

        match self.section {
            Section::GuestPort if self.guest_port.field.visual_cursor() < 65 => {
                let x = area.x + self.guest_port.field.visual_cursor() as u16 + 22;
                let y = area.y + 4;
                frame.set_cursor_position((x, y));
            }
            Section::HostPort if self.host_port.field.visual_cursor() < 50 => {
                let x = area.x + self.host_port.field.visual_cursor() as u16 + 22;
                let y = area.y + 7;
                frame.set_cursor_position((x, y));
            }
            _ => {}
        }
    }
}
