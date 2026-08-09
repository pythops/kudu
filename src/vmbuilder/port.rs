use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{ListItem, Row, Table, TableState},
};

use crossterm::event::{KeyCode, KeyEvent};

use crate::network::{MappingBuilder, PortMapping};

#[derive(Debug, Clone)]
pub struct PortForwaring {
    mappings: Vec<PortMapping>,
    mapping_state: TableState,
    new_mapping: Option<MappingBuilder>,
}

impl PortForwaring {
    pub fn new() -> Self {
        PortForwaring {
            mappings: Vec::new(),
            mapping_state: TableState::new(),
            new_mapping: None,
        }
    }

    pub fn new_mapping_popup(&self) -> bool {
        self.new_mapping.is_some()
    }

    pub fn port_mappings(&self) -> Vec<PortMapping> {
        self.mappings.clone()
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        if let Some(new_mapping) = &mut self.new_mapping {
            match key_event.code {
                KeyCode::Esc => {
                    self.new_mapping = None;
                }
                KeyCode::Enter => {
                    if new_mapping.validate() {
                        let mapping = new_mapping.build();
                        self.mappings.push(mapping);
                        self.new_mapping = None;

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
                    self.mappings.remove(index);

                    if !self.mappings.is_empty() {
                        self.mapping_state.select(Some(index.saturating_sub(1)));
                    } else {
                        self.mapping_state.select(None);
                    }
                }
            }
            KeyCode::Char('n') => {
                self.new_mapping = Some(MappingBuilder::new());
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
                        Span::from("Port Forwaring ").bold(),
                        Span::from(" ".repeat(4)),
                        Span::from(" - "),
                    ]),
                    Line::from(""),
                ]
            } else {
                lines.push(Line::from(vec![
                    Span::from("Port Forwaring ").bold(),
                    Span::from(" ".repeat(4)),
                    Span::from(format!(
                        " {} - (Guest){} <-> {}(Host)",
                        self.mappings[0].protocol,
                        self.mappings[0].guest_port,
                        self.mappings[0].host_port
                    )),
                ]));
                for mapping in self.mappings.iter().skip(1) {
                    lines.push(Line::from(vec![
                        Span::from(" ".repeat(20)),
                        Span::from(format!(
                            " {} - (Guest){} <---> {}(Host)",
                            mapping.protocol, mapping.guest_port, mapping.host_port
                        )),
                    ]))
                }
                lines.push(Line::from(""));
                lines
            }
        })]
    }
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if self.mappings.is_empty() {
            let message = Text::from("Press n to set up port forwading").centered();
            frame.render_widget(
                message,
                area.inner(Margin {
                    horizontal: 0,
                    vertical: 3,
                }),
            );
        } else {
            let widths = [
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
            ];
            let mappins = self.mappings.iter().map(|mapping| {
                Row::new(vec![
                    mapping.protocol.to_string(),
                    mapping.guest_port.to_string(),
                    mapping.host_port.to_string(),
                ])
            });

            let mappings = Table::new(mappins, widths)
                .header(
                    Row::new(vec!["Protocol", "Guest Port", "Host Port"])
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
