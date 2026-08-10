use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{ListItem, Row, Table, TableState},
};

use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    Arch,
    storage::{Disk, DiskBuilder},
};

#[derive(Debug, Clone)]
pub struct Storage {
    disks: Vec<Disk>,
    disk_state: TableState,
    new_disk: Option<DiskBuilder>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            disks: Vec::new(),
            disk_state: TableState::default(),
            new_disk: None,
        }
    }

    pub fn disks(&self) -> Vec<Disk> {
        self.disks.clone()
    }

    pub fn new_disk_popup(&self) -> bool {
        self.new_disk.is_some()
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent, arch: Arch) {
        if let Some(new_disk) = &mut self.new_disk {
            match key_event.code {
                KeyCode::Esc => {
                    self.new_disk = None;
                }
                KeyCode::Enter => {
                    if new_disk.validate() {
                        let disk = new_disk.build();
                        self.disks.push(disk);
                        self.new_disk = None;

                        if self.disk_state.selected().is_none() {
                            self.disk_state.select(Some(0));
                        }
                    }
                }

                _ => {
                    new_disk.handle_key_events(key_event, arch);
                }
            }

            return;
        }
        match key_event.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(index) = self.disk_state.selected() {
                    self.disk_state
                        .select(Some(index.saturating_add(1).min(self.disks.len() - 1)));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(index) = self.disk_state.selected() {
                    self.disk_state.select(Some(index.saturating_sub(1)));
                }
            }
            KeyCode::Char('d') => {
                if let Some(index) = self.disk_state.selected() {
                    self.disks.remove(index);

                    if !self.disks.is_empty() {
                        self.disk_state.select(Some(index.saturating_sub(1)));
                    } else {
                        self.disk_state.select(None);
                    }
                }
            }
            KeyCode::Char('n') => {
                self.new_disk = Some(DiskBuilder::new());
            }
            _ => {}
        }
    }

    pub fn summary(&self) -> Vec<ListItem<'_>> {
        vec![ListItem::from({
            let mut lines = Vec::new();
            if self.disks.is_empty() {
                vec![
                    Line::from(vec![
                        Span::from("Additional Disks ").bold(),
                        Span::from(" ".repeat(2)),
                        Span::from(" - "),
                    ]),
                    Line::from(""),
                ]
            } else {
                lines.push(Line::from(vec![
                    Span::from("Disks  ").bold(),
                    Span::from(" ".repeat(14)),
                    Span::from("     Size   ").bold(),
                    Span::from(" ".repeat(4)),
                    Span::from(" Format ").bold(),
                    Span::from(" ".repeat(4)),
                    Span::from(" Interface ").bold(),
                ]));
                lines.push(Line::from(""));
                for (index, disk) in self.disks.iter().enumerate() {
                    lines.push(Line::from(vec![
                        Span::from(" ".repeat(20)),
                        Span::from(index.to_string()),
                        Span::from(" ".repeat(3)),
                        Span::from(format!("{:3}GiB", disk.size)),
                        Span::from(" ".repeat(8)),
                        Span::from(disk.format.to_string()),
                        Span::from(" ".repeat(8)),
                        Span::from(disk.interface.to_string()),
                    ]))
                }
                lines.push(Line::from(""));
                lines
            }
        })]
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if self.disks.is_empty() {
            let message = Text::from("Press n to add additional disks").centered();
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
                Constraint::Length(15),
                Constraint::Length(15),
                Constraint::Length(15),
            ];
            let disks = self.disks.iter().enumerate().map(|(index, disk)| {
                Row::new(vec![
                    index.to_string(),
                    disk.format.to_string(),
                    format!("{} GiB", disk.size),
                    disk.interface.to_string(),
                ])
            });

            let disks = Table::new(disks, widths)
                .header(
                    Row::new(vec!["", "Format", "Size", "Interface"])
                        .style(Style::new().bold())
                        .bottom_margin(1),
                )
                .flex(ratatui::layout::Flex::SpaceBetween)
                .row_highlight_style(Style::new().on_dark_gray())
                .column_spacing(1);

            frame.render_stateful_widget(
                disks,
                area.inner(Margin {
                    horizontal: 2,
                    vertical: 2,
                }),
                &mut self.disk_state,
            );
        }
        if let Some(new_disk) = &self.new_disk {
            new_disk.render(frame);
        }
    }
}
