use std::{cell::RefCell, rc::Rc};

use crossterm::event::{KeyCode, KeyEvent};
use rand::RngExt;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, ListItem, Row, Table, TableState},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{KUDU_BRIDGE_INTERFACE, USER_UID};

use super::{Network, NetworkBackend, Nic};

#[derive(Debug, Clone)]
pub struct NetworkBuilder {
    networks: Rc<RefCell<Vec<Network>>>,
    new_network: Option<NewNetwork>,
    netowrk_state: TableState,
}

impl Default for NetworkBuilder {
    fn default() -> Self {
        Self {
            networks: Rc::new(RefCell::new(vec![Network::default()])),
            new_network: None,
            netowrk_state: TableState::default().with_selected(Some(0)),
        }
    }
}

impl NetworkBuilder {
    pub fn new() -> NetworkBuilder {
        NetworkBuilder::default()
    }

    pub fn new_network_popup(&self) -> bool {
        self.new_network.is_some()
    }

    pub fn networks(&self) -> Rc<RefCell<Vec<Network>>> {
        self.networks.clone()
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        if let Some(new_network) = &mut self.new_network {
            match key_event.code {
                KeyCode::Esc => {
                    self.new_network = None;
                }
                KeyCode::Enter => {
                    if new_network.validate() {
                        let network = new_network.build();

                        self.networks.borrow_mut().push(network);
                        self.new_network = None;

                        if self.netowrk_state.selected().is_none() {
                            self.netowrk_state.select(Some(0));
                        }
                    }
                }

                _ => {
                    new_network.handle_key_events(key_event);
                }
            }

            return;
        }

        match key_event.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(index) = self.netowrk_state.selected() {
                    self.netowrk_state.select(Some(
                        index
                            .saturating_add(1)
                            .min(self.networks.borrow().len() - 1),
                    ));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(index) = self.netowrk_state.selected() {
                    self.netowrk_state.select(Some(index.saturating_sub(1)));
                }
            }
            KeyCode::Char('d') => {
                if let Some(index) = self.netowrk_state.selected() {
                    let mut networks = self.networks.borrow_mut();
                    networks.remove(index);

                    if !networks.is_empty() {
                        self.netowrk_state.select(Some(index.saturating_sub(1)));
                    } else {
                        self.netowrk_state.select(None);
                    }
                }
            }
            KeyCode::Char('n') => {
                self.new_network = Some(NewNetwork::new());
            }
            _ => {}
        }
    }

    pub fn summary(&self) -> Vec<ListItem<'_>> {
        vec![ListItem::from({
            let mut lines = Vec::new();
            let networks = self.networks.borrow();
            if networks.is_empty() {
                vec![
                    Line::from(vec![
                        Span::from("Networks").bold(),
                        Span::from(" ".repeat(13)),
                        Span::from(" - "),
                    ]),
                    Line::from(""),
                ]
            } else {
                lines.push(Line::from(vec![
                    Span::from("Networks").bold(),
                    Span::from(" ".repeat(12)),
                    Span::from("Id   ").bold(),
                    Span::from(" ".repeat(8)),
                    Span::from("Backend").bold(),
                    Span::from(" ".repeat(12)),
                    Span::from("  Nic  ").bold(),
                    Span::from(" ".repeat(14)),
                    Span::from("  Mac  ").bold(),
                ]));
                lines.push(Line::from(""));
                for network in networks.iter() {
                    lines.push(Line::from(vec![
                        Span::from(" ".repeat(20)),
                        Span::from(format!("{:8}", network.id)),
                        Span::from(" ".repeat(5)),
                        Span::from(format!("{:14}", network.backend)),
                        Span::from(" ".repeat(7)),
                        Span::from(format!("{:14}", network.nic)),
                        Span::from(" ".repeat(7)),
                        Span::from(format!(
                            "{:17}",
                            network.mac.clone().unwrap_or("Auto".to_string())
                        )),
                    ]))
                }
                lines.push(Line::from(""));
                lines
            }
        })]
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let networks = self.networks.borrow();
        if networks.is_empty() {
            let area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Length(1),
                    Constraint::Fill(1),
                ])
                .flex(Flex::Center)
                .split(area)[1];
            let message = Text::from("Press n to add a network interface").centered();
            frame.render_widget(message, area);
        } else {
            let widths = [
                Constraint::Length(8),  // id
                Constraint::Length(14), // backend
                Constraint::Length(10), // Nic
                Constraint::Length(17), // Mac
            ];

            let networks = networks.iter().map(|network| {
                Row::new(vec![
                    network.id.to_string(),
                    network.backend.to_string(),
                    network.nic.to_string(),
                    network.mac.clone().unwrap_or("Auto".to_string()),
                ])
            });

            let networks = Table::new(networks, widths)
                .header(
                    Row::new(vec!["id", "Backend", "Nic", "Mac"])
                        .style(Style::new().bold())
                        .bottom_margin(1),
                )
                .flex(ratatui::layout::Flex::SpaceBetween)
                .row_highlight_style(Style::new().on_dark_gray())
                .column_spacing(1);

            frame.render_stateful_widget(
                networks,
                area.inner(Margin {
                    horizontal: 2,
                    vertical: 2,
                }),
                &mut self.netowrk_state,
            );
        }
        if let Some(new_network) = &self.new_network {
            new_network.render(frame);
        }
    }
}

// New Network

#[derive(Debug, Clone, Default)]
struct UserInputField {
    field: Input,
    error: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum Section {
    #[default]
    Backend,
    Nic,
    Mac,
}

#[derive(Debug, Clone, Default)]
pub struct NewNetwork {
    section: Section,
    backend: NetworkBackend,
    nic: Nic,
    mac: UserInputField,
}

fn random_mac() -> String {
    let mut mac: [u8; 6] = rand::rng().random();
    mac[0] = (mac[0] & 0xFC) | 0x02;
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

impl NewNetwork {
    pub fn new() -> Self {
        NewNetwork::default()
    }

    fn mac(&self) -> Option<String> {
        if self.mac.field.value().is_empty() {
            None
        } else {
            Some(self.mac.field.value().to_string())
        }
    }

    pub fn build(&self) -> Network {
        match self.backend {
            NetworkBackend::Tap | NetworkBackend::Bridge(_) if self.mac().is_none() => {
                Network::new(
                    self.backend.clone(),
                    self.nic,
                    Vec::new(),
                    Some(random_mac()),
                )
            }
            _ => Network::new(self.backend.clone(), self.nic, Vec::new(), self.mac()),
        }
    }

    pub fn validate(&mut self) -> bool {
        let mut valid = true;

        if !self.mac.field.value().is_empty() {
            if self.mac.field.value().len() != 17 {
                valid = false
            } else {
                for (index, c) in self.mac.field.value().chars().enumerate() {
                    if index % 3 == 2 {
                        if c != ':' {
                            println!("{}, index {}", c, index);
                            break;
                        }
                    } else {
                        if !c.is_ascii_hexdigit() {
                            valid = false;
                            break;
                        }
                    }
                }
            }
        }

        if !valid {
            self.mac.error = Some("Unvalid MAC address".into());
        }

        valid
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Down | KeyCode::Char('j') => match self.section {
                Section::Backend => self.section = Section::Nic,
                Section::Nic => self.section = Section::Mac,
                Section::Mac => self.section = Section::Backend,
            },
            KeyCode::Up | KeyCode::Char('k') => match self.section {
                Section::Backend => self.section = Section::Mac,
                Section::Nic => self.section = Section::Backend,
                Section::Mac => self.section = Section::Nic,
            },

            _ => match self.section {
                Section::Backend => match key_event.code {
                    KeyCode::Right | KeyCode::Char('l') => match self.backend {
                        NetworkBackend::Passt => {
                            self.backend = NetworkBackend::User;
                        }
                        NetworkBackend::User => {
                            if unsafe { USER_UID == 0 } {
                                self.backend = NetworkBackend::Tap;
                            } else {
                                self.backend = NetworkBackend::Passt;
                            }
                        }
                        NetworkBackend::Tap => {
                            self.backend = NetworkBackend::Bridge(KUDU_BRIDGE_INTERFACE.into());
                        }
                        NetworkBackend::Bridge(_) => {
                            self.backend = NetworkBackend::Passt;
                        }
                    },
                    KeyCode::Left | KeyCode::Char('h') => match self.backend {
                        NetworkBackend::Passt => {
                            if unsafe { USER_UID == 0 } {
                                self.backend = NetworkBackend::Bridge(KUDU_BRIDGE_INTERFACE.into());
                            } else {
                                self.backend = NetworkBackend::User;
                            }
                        }
                        NetworkBackend::User => {
                            self.backend = NetworkBackend::Passt;
                        }
                        NetworkBackend::Tap => {
                            self.backend = NetworkBackend::User;
                        }
                        NetworkBackend::Bridge(_) => {
                            self.backend = NetworkBackend::Tap;
                        }
                    },
                    _ => {}
                },

                Section::Nic => match key_event.code {
                    KeyCode::Right | KeyCode::Char('l') => match self.nic {
                        Nic::Virtio => {
                            self.nic = Nic::E1000;
                        }
                        Nic::E1000 => {
                            self.nic = Nic::RTL8139;
                        }
                        Nic::RTL8139 => {
                            self.nic = Nic::Virtio;
                        }
                    },
                    KeyCode::Left | KeyCode::Char('h') => match self.nic {
                        Nic::Virtio => {
                            self.nic = Nic::RTL8139;
                        }
                        Nic::E1000 => {
                            self.nic = Nic::Virtio;
                        }
                        Nic::RTL8139 => {
                            self.nic = Nic::E1000;
                        }
                    },
                    _ => {}
                },
                Section::Mac => {
                    self.mac
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
            },
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(16),
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
                .title(" New Network 󰛳  ")
                .border_type(BorderType::Thick)
                .title_alignment(ratatui::layout::HorizontalAlignment::Center)
                .border_style(Style::default().yellow()),
            area,
        );

        let area = area.inner(Margin {
            horizontal: 2,
            vertical: 2,
        });

        let (backend_block, nic_block, mac_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Length(2),
                    Constraint::Length(2),
                ])
                .margin(1)
                .flex(ratatui::layout::Flex::SpaceBetween)
                .split(area);

            (chunks[0], chunks[1], chunks[2])
        };

        let backend = Line::from(vec![
            {
                if self.section == Section::Backend {
                    Span::from("> Backend  ").bold()
                } else {
                    Span::from("  Backend  ")
                }
            },
            Span::from(" ".repeat(4)),
            Span::from(format!("< {} >", self.backend)),
        ]);

        let nic = Line::from(vec![
            {
                if self.section == Section::Nic {
                    Span::from("> Nic ").bold()
                } else {
                    Span::from("  Nic ")
                }
            },
            Span::from(" ".repeat(9)),
            Span::from(format!("< {} >", self.nic)),
        ]);

        let mac = vec![
            Line::from(vec![
                {
                    if self.section == Section::Mac {
                        Span::from("> Mac ").bold()
                    } else {
                        Span::from("  Mac ")
                    }
                },
                Span::from(" ".repeat(9)),
                Span::from({
                    let original_length = self.mac.field.to_string().len();
                    let target_length = 30_usize;

                    self.mac
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
                Span::from(" ".repeat(15)),
                Span::from(self.mac.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];

        frame.render_widget(Text::from(backend), backend_block);
        frame.render_widget(Text::from(nic), nic_block);
        frame.render_widget(Text::from(mac), mac_block);

        if self.section == Section::Mac && self.mac.field.visual_cursor() < 40 {
            let x = area.x + self.mac.field.visual_cursor() as u16 + 16;
            let y = area.y + 7;
            frame.set_cursor_position((x, y));
        }
    }
}
