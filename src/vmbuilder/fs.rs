use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{ListItem, Row, Table, TableState},
};

use crossterm::event::{KeyCode, KeyEvent};

use crate::fs::{Filesystem, NewFs};

#[derive(Debug, Clone, Default)]
pub struct FsBuilder {
    fs: Vec<Filesystem>,
    fs_state: TableState,
    new_fs: Option<NewFs>,
}

impl FsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_fs_popup(&self) -> bool {
        self.new_fs.is_some()
    }

    pub fn filesystems(&self) -> Vec<Filesystem> {
        self.fs.clone()
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        if let Some(new_fs) = &mut self.new_fs {
            match key_event.code {
                KeyCode::Esc => {
                    self.new_fs = None;
                }
                KeyCode::Enter => {
                    if new_fs.validate() {
                        let fs = new_fs.build();

                        self.fs.push(fs);
                        self.new_fs = None;

                        if self.fs_state.selected().is_none() {
                            self.fs_state.select(Some(0));
                        }
                    }
                }

                _ => {
                    new_fs.handle_key_events(key_event);
                }
            }

            return;
        }

        match key_event.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(index) = self.fs_state.selected() {
                    self.fs_state
                        .select(Some(index.saturating_add(1).min(self.fs.len() - 1)));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(index) = self.fs_state.selected() {
                    self.fs_state.select(Some(index.saturating_sub(1)));
                }
            }
            KeyCode::Char('d') => {
                if let Some(index) = self.fs_state.selected() {
                    self.fs.remove(index);
                }
            }
            KeyCode::Char('n') => {
                self.new_fs = Some(NewFs::new());
            }
            _ => {}
        }
    }

    pub fn summary(&self) -> Vec<ListItem<'_>> {
        vec![ListItem::from({
            let mut lines = Vec::new();

            if self.fs.is_empty() {
                vec![
                    Line::from(vec![
                        Span::from("Filesystem").bold(),
                        Span::from(" ".repeat(9)),
                        Span::from(" - "),
                    ]),
                    Line::from(""),
                ]
            } else {
                lines.push(Line::from(vec![
                    Span::from("Filesystem").bold(),
                    Span::from(" ".repeat(10)),
                    Span::from("Source Path ").bold(),
                    Span::from(" ".repeat(10)),
                    Span::from(" Mount Tag ").bold(),
                    Span::from(" ".repeat(12)),
                    Span::from(" Readonly ").bold(),
                    Span::from(" ".repeat(2)),
                    Span::from(" Drive ").bold(),
                ]));
                lines.push(Line::from(""));
                for fs in &self.fs {
                    lines.push(Line::from(vec![
                        Span::from(" ".repeat(20)),
                        Span::from(format!("{:23}", fs.source_path.to_string_lossy())),
                        Span::from(format!("{:15}", fs.mount_tag)),
                        Span::from(" ".repeat(8)),
                        Span::from(format!("{:3}", if fs.readonly { "Yes" } else { "No" })),
                        Span::from(" ".repeat(9)),
                        Span::from(fs.driver.to_string()),
                    ]))
                }
                lines.push(Line::from(""));
                lines
            }
        })]
    }
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if self.fs.is_empty() {
            let area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Length(1),
                    Constraint::Fill(1),
                ])
                .flex(Flex::Center)
                .split(area)[1];

            let message = Text::from("Press n to add a shared fs").centered();
            frame.render_widget(message, area);
        } else {
            let widths = [
                Constraint::Length(20),
                Constraint::Length(20),
                Constraint::Length(8),
                Constraint::Length(10),
            ];
            let fs = self.fs.iter().map(|fs| {
                Row::new(vec![
                    fs.source_path.to_string_lossy().to_string(),
                    fs.mount_tag.clone(),
                    {
                        if fs.readonly {
                            "Yes".to_string()
                        } else {
                            "No".to_string()
                        }
                    },
                    fs.driver.to_string(),
                ])
            });

            let fs = Table::new(fs, widths)
                .header(
                    Row::new(vec!["Source Path", "Mount Tag", "Readonly", "Driver"])
                        .style(Style::new().bold())
                        .bottom_margin(1),
                )
                .flex(ratatui::layout::Flex::SpaceAround)
                .row_highlight_style(Style::new().on_dark_gray())
                .column_spacing(1);

            frame.render_stateful_widget(
                fs,
                area.inner(Margin {
                    horizontal: 2,
                    vertical: 2,
                }),
                &mut self.fs_state,
            );
        }

        if let Some(new_fs) = &self.new_fs {
            new_fs.render(frame);
        }
    }
}
