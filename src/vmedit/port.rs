use std::{cell::RefCell, rc::Rc};

use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    style::{Style, Stylize},
    text::Text,
    widgets::{Row, Table, TableState},
};

use crate::network::{
    Network, NetworkId,
    port_forwarding::{MappingBuilder, PortMapping},
};

#[derive(Debug, Clone)]
pub(super) struct PortForwarding {
    added_port_mappings: Vec<(NetworkId, PortMapping)>,
    deleted_port_mappings: Vec<(NetworkId, PortMapping)>,
    new_mapping: Option<MappingBuilder>,
    mapping_state: TableState,
    networks: Rc<RefCell<Vec<Network>>>,
}

impl PortForwarding {
    pub fn new(networks: Rc<RefCell<Vec<Network>>>) -> Self {
        let mut mappings = Vec::new();

        for network in networks.borrow().iter() {
            for mapping in network.port_mappings.clone() {
                mappings.push((network.id.clone(), mapping));
            }
        }
        let mapping_state = if mappings.is_empty() {
            TableState::default()
        } else {
            TableState::default().with_selected(Some(0))
        };

        Self {
            added_port_mappings: Vec::new(),
            deleted_port_mappings: Vec::new(),
            new_mapping: None,
            mapping_state,
            networks,
        }
    }

    pub fn refresh(&mut self) {
        let netowrk_ids: Vec<NetworkId> = self
            .networks
            .borrow()
            .iter()
            .map(|n| n.id.clone())
            .collect();

        for mapping in &self.added_port_mappings.clone() {
            if !netowrk_ids.contains(&mapping.0) {
                self.added_port_mappings.retain(|m| m != mapping);
            }
        }
    }

    fn mappings(&self) -> Vec<(NetworkId, PortMapping)> {
        let mut mappings = Vec::new();

        for network in self.networks.borrow().iter() {
            for mapping in network.port_mappings.clone() {
                mappings.push((network.id.clone(), mapping));
            }
        }

        mappings
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
                        self.added_port_mappings.push(mapping);
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

        let vm_mappings = self.mappings();

        match key_event.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(index) = self.mapping_state.selected() {
                    self.mapping_state.select(Some(
                        index
                            .saturating_add(1)
                            .min(vm_mappings.len() + self.added_port_mappings.len() - 1),
                    ));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(index) = self.mapping_state.selected() {
                    self.mapping_state.select(Some(index.saturating_sub(1)));
                }
            }
            KeyCode::Char('d') => {
                if let Some(index) = self.mapping_state.selected() {
                    if index < vm_mappings.len() {
                        let mapping = &vm_mappings[index];
                        if !self.deleted_port_mappings.contains(mapping) {
                            self.deleted_port_mappings.push(mapping.clone());
                        }
                    } else {
                        let index = index.saturating_sub(vm_mappings.len());
                        self.added_port_mappings.remove(index);
                    }
                }
            }
            KeyCode::Char('u') => {
                if let Some(index) = self.mapping_state.selected()
                    && let Some(mapping) = vm_mappings.get(index)
                {
                    self.deleted_port_mappings
                        .retain(|deleted_mapping| deleted_mapping != mapping);
                }
            }
            KeyCode::Char('n') if !self.networks.borrow().is_empty() => {
                self.new_mapping = Some(MappingBuilder::new(self.networks.clone()));
            }
            _ => {}
        }
    }

    pub fn new_mapping_popup(&self) -> bool {
        self.new_mapping.is_some()
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let vm_mappings = self.mappings();
        let area = area.inner(Margin {
            horizontal: 2,
            vertical: 2,
        });

        if vm_mappings.is_empty() && self.added_port_mappings.is_empty() {
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
                Constraint::Length(5),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
            ];

            let vm_port_mappings = vm_mappings.iter().map(|(network_id, mapping)| {
                let to_delete = self
                    .deleted_port_mappings
                    .contains(&(network_id.clone(), *mapping));
                Row::new(vec![
                    {
                        if to_delete {
                            "Del".to_string()
                        } else {
                            String::new()
                        }
                    },
                    network_id.to_string(),
                    mapping.protocol.to_string(),
                    mapping.guest_port.to_string(),
                    mapping.host_port.to_string(),
                ])
                .style(if to_delete {
                    Style::new().red()
                } else {
                    Style::default()
                })
            });

            let new_port_mappings = self
                .added_port_mappings
                .iter()
                .map(|(network_id, mapping)| {
                    Row::new(vec![
                        "New".to_string(),
                        network_id.to_string(),
                        mapping.protocol.to_string(),
                        mapping.guest_port.to_string(),
                        mapping.host_port.to_string(),
                    ])
                    .green()
                });

            let mut mappings: Vec<Row> = Vec::new();
            mappings.extend(vm_port_mappings);
            mappings.extend(new_port_mappings);

            let mappings = Table::new(mappings, widths)
                .header(
                    Row::new(vec!["", "Network", "Protocol", "Guest Port", "Host Port"])
                        .style(Style::new().bold())
                        .bottom_margin(1),
                )
                .flex(ratatui::layout::Flex::SpaceBetween)
                .row_highlight_style(Style::new().on_dark_gray())
                .column_spacing(1);

            frame.render_stateful_widget(mappings, area, &mut self.mapping_state);
        }
        if let Some(new_mapping) = &self.new_mapping {
            new_mapping.render(frame);
        }
    }
}
