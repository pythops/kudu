use std::{cell::RefCell, rc::Rc};

use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    style::{Style, Stylize},
    text::Text,
    widgets::{Row, Table, TableState},
};

use crate::network::{Network, builder::NewNetwork};

#[derive(Debug, Clone)]
pub(super) struct NetworkEdit {
    added_networks: Vec<Network>,
    deleted_networks: Vec<Network>,
    new_network: Option<NewNetwork>,
    network_state: TableState,
    initial_networks: Vec<Network>,
    networks: Rc<RefCell<Vec<Network>>>,
}

impl NetworkEdit {
    pub fn new(networks: Rc<RefCell<Vec<Network>>>) -> Self {
        let initial_networks = networks.borrow().clone();

        let network_state = if initial_networks.is_empty() {
            TableState::default()
        } else {
            TableState::default().with_selected(Some(0))
        };

        Self {
            added_networks: Vec::new(),
            deleted_networks: Vec::new(),
            new_network: None,
            network_state,
            initial_networks,
            networks,
        }
    }

    pub fn build(&self) -> Vec<Network> {
        self.networks.take()
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

                        self.networks.borrow_mut().push(network.clone());
                        self.added_networks.push(network);
                        self.new_network = None;

                        if self.network_state.selected().is_none() {
                            self.network_state.select(Some(0));
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
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(index) = self.network_state.selected() {
                    self.network_state.select(Some(
                        index
                            .saturating_add(1)
                            .min(self.initial_networks.len() + self.added_networks.len() - 1),
                    ));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(index) = self.network_state.selected() {
                    self.network_state.select(Some(index.saturating_sub(1)));
                }
            }
            KeyCode::Char('d') => {
                if let Some(index) = self.network_state.selected() {
                    if index < self.initial_networks.len() {
                        let network = &self.initial_networks[index];
                        if !self.deleted_networks.contains(network) {
                            self.deleted_networks.push(network.clone());
                        }
                        self.networks.borrow_mut().retain(|n| n != network);
                    } else {
                        let index = index.saturating_sub(self.initial_networks.len());
                        let network = self.added_networks.remove(index);
                        self.networks.borrow_mut().retain(|n| n != &network);
                    }
                }
            }
            KeyCode::Char('u') => {
                if let Some(index) = self.network_state.selected()
                    && let Some(network) = self.initial_networks.get(index)
                {
                    self.deleted_networks
                        .retain(|deleted_network| deleted_network != network);
                    self.networks.borrow_mut().push(network.clone());
                }
            }
            KeyCode::Char('n') if !self.networks.borrow().is_empty() => {
                self.new_network = Some(NewNetwork::new());
            }
            _ => {}
        }
    }

    pub fn new_network_popup(&self) -> bool {
        self.new_network.is_some()
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(Margin {
            horizontal: 2,
            vertical: 2,
        });

        if self.initial_networks.is_empty() && self.added_networks.is_empty() {
            let message = Text::from("Press n to add a network interface").centered();
            frame.render_widget(
                message,
                area.inner(Margin {
                    horizontal: 0,
                    vertical: 3,
                }),
            );
        } else {
            let widths = [
                Constraint::Length(5),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(17),
            ];

            let initial_networks = self.initial_networks.iter().map(|network| {
                let to_delete = self.deleted_networks.contains(network);
                Row::new(vec![
                    {
                        if to_delete {
                            "Del".to_string()
                        } else {
                            String::new()
                        }
                    },
                    network.id.to_string(),
                    network.backend.to_string(),
                    network.nic.to_string(),
                    network.mac.clone().unwrap_or("Auto".to_string()),
                ])
                .style(if to_delete {
                    Style::new().red()
                } else {
                    Style::default()
                })
            });

            let new_networks = self.added_networks.iter().map(|network| {
                Row::new(vec![
                    "New".to_string(),
                    network.id.to_string(),
                    network.backend.to_string(),
                    network.nic.to_string(),
                    network.mac.clone().unwrap_or("Auto".to_string()),
                ])
                .green()
            });

            let mut networks: Vec<Row> = Vec::new();
            networks.extend(initial_networks);
            networks.extend(new_networks);

            let networks = Table::new(networks, widths)
                .header(
                    Row::new(vec!["", "id", "Backend", "Nic", "Mac"])
                        .style(Style::new().bold())
                        .bottom_margin(1),
                )
                .flex(ratatui::layout::Flex::SpaceBetween)
                .row_highlight_style(Style::new().on_dark_gray())
                .column_spacing(1);
            frame.render_stateful_widget(networks, area, &mut self.network_state);
        }
        if let Some(new_network) = &self.new_network {
            new_network.render(frame);
        }
    }
}
