use std::{cell::RefCell, rc::Rc};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{ListItem, Row, Table, TableState},
};

use crossterm::event::{KeyCode, KeyEvent};

use crate::network::{
    Network, NetworkId,
    port_forwarding::{MappingBuilder, PortMapping},
};

#[derive(Debug, Clone)]
pub struct PortForwarding {
    mappings: Vec<(NetworkId, PortMapping)>,
    mapping_state: TableState,
    new_mapping: Option<MappingBuilder>,
    networks: Rc<RefCell<Vec<Network>>>,
}

impl PortForwarding {
    pub fn new(networks: Rc<RefCell<Vec<Network>>>) -> Self {
        PortForwarding {
            mappings: Vec::new(),
            mapping_state: TableState::new(),
            new_mapping: None,
            networks,
        }
    }

    pub fn new_mapping_popup(&self) -> bool {
        self.new_mapping.is_some()
    }

    pub fn refresh(&mut self) {
        let mut mappings = Vec::new();

        for network in self.networks.borrow().iter() {
            for mapping in network.port_mappings.clone() {
                mappings.push((network.id.clone(), mapping));
            }
        }

        self.mappings = mappings;
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        if let Some(new_mapping) = &mut self.new_mapping {
            match key_event.code {
                KeyCode::Esc => {
                    self.new_mapping = None;
                }
                KeyCode::Enter => {
                    if new_mapping.validate() {
                        let (network_id, mapping) = new_mapping.build();

                        if let Some(network) = self
                            .networks
                            .borrow_mut()
                            .iter_mut()
                            .find(|network| network.id == network_id)
                        {
                            network.port_mappings.push(mapping);
                        }

                        self.new_mapping = None;
                        self.refresh();

                        if self.mapping_state.selected().is_none() {
                            self.mapping_state.select(Some(0));
                        }
                    }
                }

                _ => {
                    new_mapping.handle_key_events(key_event);
                }
            }
            return;
        }

        match key_event.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(index) = self.mapping_state.selected() {
                    self.mapping_state
                        .select(Some(index.saturating_add(1).min(self.mappings.len() - 1)));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(index) = self.mapping_state.selected() {
                    self.mapping_state.select(Some(index.saturating_sub(1)));
                }
            }
            KeyCode::Char('d') => {
                if let Some(index) = self.mapping_state.selected() {
                    let (netowrk_id, mapping) = self.mappings.remove(index);

                    if let Some(network) = self
                        .networks
                        .borrow_mut()
                        .iter_mut()
                        .find(|network| network.id == netowrk_id)
                    {
                        network.port_mappings.retain(|&m| m != mapping);
                    }

                    self.refresh();

                    if !self.mappings.is_empty() {
                        self.mapping_state.select(Some(index.saturating_sub(1)));
                    } else {
                        self.mapping_state.select(None);
                    }
                }
            }
            KeyCode::Char('n') if !self.networks.borrow().is_empty() => {
                self.new_mapping = Some(MappingBuilder::new(self.networks.clone()));
            }
            _ => {}
        }
    }

    pub fn summary(&self) -> Vec<ListItem<'_>> {
        vec![ListItem::from({
            let mut lines = Vec::new();
            if self.mappings.is_empty() {
                vec![
                    Line::from(vec![
                        Span::from("Port Forwarding ").bold(),
                        Span::from(" ".repeat(4)),
                        Span::from(" - "),
                    ]),
                    Line::from(""),
                ]
            } else {
                lines.push(Line::from(vec![
                    Span::from("Port Forwarding ").bold(),
                    Span::from(" ".repeat(4)),
                    Span::from(format!(
                        "{}  --  {} - (Guest){} <-> {}(Host)",
                        self.mappings[0].0,
                        self.mappings[0].1.protocol,
                        self.mappings[0].1.guest_port,
                        self.mappings[0].1.host_port
                    )),
                ]));
                for mapping in self.mappings.iter().skip(1) {
                    lines.push(Line::from(vec![
                        Span::from(" ".repeat(20)),
                        Span::from(format!(
                            "{}  --  {} - (Guest){} <---> {}(Host)",
                            mapping.0,
                            mapping.1.protocol,
                            mapping.1.guest_port,
                            mapping.1.host_port
                        )),
                    ]))
                }
                lines.push(Line::from(""));
                lines
            }
        })]
    }
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if self.networks.borrow().is_empty() {
            let area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Length(1),
                    Constraint::Fill(1),
                ])
                .flex(Flex::Center)
                .split(area)[1];
            let message =
                Text::from("Create a network first before setting up port forwarding").centered();
            frame.render_widget(message, area);
        } else if self.mappings.is_empty() {
            let area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Length(1),
                    Constraint::Fill(1),
                ])
                .flex(Flex::Center)
                .split(area)[1];
            let message = Text::from("Press n to set up port forwarding").centered();
            frame.render_widget(message, area);
        } else {
            let widths = [
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
            ];
            let mappins = self.mappings.iter().map(|mapping| {
                Row::new(vec![
                    mapping.0.clone(),
                    mapping.1.protocol.to_string(),
                    mapping.1.guest_port.to_string(),
                    mapping.1.host_port.to_string(),
                ])
            });

            let mappings = Table::new(mappins, widths)
                .header(
                    Row::new(vec!["Network", "Protocol", "Guest Port", "Host Port"])
                        .style(Style::new().bold())
                        .bottom_margin(1),
                )
                .flex(ratatui::layout::Flex::SpaceBetween)
                .row_highlight_style(Style::new().on_dark_gray())
                .column_spacing(1);

            frame.render_stateful_widget(
                mappings,
                area.inner(Margin {
                    horizontal: 2,
                    vertical: 2,
                }),
                &mut self.mapping_state,
            );
        }

        if let Some(new_mapping) = &self.new_mapping {
            new_mapping.render(frame);
        }
    }
}
