use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Row, Table, TableState},
};

use crate::fs::{Filesystem, NewFs};

#[derive(Debug, Clone)]
pub struct FsEdit {
    fs_state: TableState,
    new_fs: Option<NewFs>,
    added_fs: Vec<Filesystem>,
    deleted_fs: Vec<Filesystem>,
    fs: Vec<Filesystem>,
}

impl FsEdit {
    pub fn new(fs: Vec<Filesystem>) -> Self {
        let fs_state = if fs.is_empty() {
            TableState::default()
        } else {
            TableState::default().with_selected(Some(0))
        };

        Self {
            fs_state,
            new_fs: None,
            added_fs: Vec::new(),
            deleted_fs: Vec::new(),
            fs,
        }
    }

    pub fn new_fs_popup(&self) -> bool {
        self.new_fs.is_some()
    }

    pub fn build(&self) -> Vec<Filesystem> {
        let mut fs = self.fs.clone();

        for deleted_fs in &self.deleted_fs {
            fs.retain(|f| f != deleted_fs);
        }

        fs.extend(self.added_fs.clone());

        fs
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

                        self.added_fs.push(fs);
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
                    self.fs_state.select(Some(
                        index
                            .saturating_add(1)
                            .min(self.fs.len() + self.added_fs.len() - 1),
                    ));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(index) = self.fs_state.selected() {
                    self.fs_state.select(Some(index.saturating_sub(1)));
                }
            }
            KeyCode::Char('d') => {
                if let Some(index) = self.fs_state.selected() {
                    if index < self.fs.len() {
                        let fs = &self.fs[index];
                        self.deleted_fs.push(fs.clone());
                    } else {
                        let index = index.saturating_sub(self.fs.len());
                        self.added_fs.remove(index);
                    }
                }
            }
            KeyCode::Char('u') => {
                if let Some(index) = self.fs_state.selected()
                    && let Some(fs) = self.fs.get(index)
                {
                    self.deleted_fs.retain(|deleted_fs| deleted_fs != fs);
                }
            }
            KeyCode::Char('n') => {
                self.new_fs = Some(NewFs::new());
            }
            _ => {}
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let widths = [
            Constraint::Length(6),
            Constraint::Length(20),
            Constraint::Length(20),
            Constraint::Length(8),
            Constraint::Length(10),
        ];

        let vm_fs = self.fs.iter().map(|fs| {
            let to_delete = self.deleted_fs.contains(fs);
            Row::new(vec![
                Line::from({
                    if to_delete {
                        "Delete".to_string()
                    } else {
                        String::new()
                    }
                }),
                Line::from(fs.source_path.to_string_lossy().to_string()),
                Line::from(fs.mount_tag.clone()),
                Line::from({
                    if fs.readonly {
                        "Yes".to_string()
                    } else {
                        "No".to_string()
                    }
                }),
                Line::from(fs.driver.to_string()),
            ])
            .style(if to_delete {
                Style::new().red()
            } else {
                Style::default()
            })
        });

        let added_fs = self.added_fs.iter().map(|fs| {
            Row::new(vec![
                Line::from("New"),
                Line::from(fs.source_path.to_string_lossy().to_string()),
                Line::from(fs.mount_tag.clone()),
                Line::from({
                    if fs.readonly {
                        "Yes".to_string()
                    } else {
                        "No".to_string()
                    }
                }),
                Line::from(fs.driver.to_string()),
            ])
            .green()
        });

        let mut fs = Vec::new();

        fs.extend(vm_fs);
        fs.extend(added_fs);

        let fs = Table::new(fs, widths)
            .header(
                Row::new(vec!["", "Source Path", "Mount Tag", "Readonly", "Driver"])
                    .style(Style::new().bold())
                    .bottom_margin(1),
            )
            .flex(ratatui::layout::Flex::SpaceAround)
            .row_highlight_style(Style::new().on_dark_gray())
            .column_spacing(1);

        frame.render_stateful_widget(fs, area, &mut self.fs_state);

        if let Some(new_fs) = &self.new_fs {
            new_fs.render(frame);
        }
    }

    pub fn help(&self) -> Vec<Line<'static>> {
        vec![Line::from(vec![
            Span::from("↑").bold(),
            Span::from("  Up"),
            Span::from(" | "),
            Span::from("↓").bold(),
            Span::from("  Down"),
            Span::from(" | "),
            Span::from("h,←").bold(),
            Span::from("  Left"),
            Span::from(" | "),
            Span::from("l,→").bold(),
            Span::from("  Right"),
            Span::from(" | "),
            Span::from("Esc").bold(),
            Span::from(" Cancel"),
            Span::from(" | "),
            Span::from("Enter").bold(),
            Span::from(" Confirm"),
        ])]
    }
}
