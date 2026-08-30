use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Style, Stylize},
    widgets::{Row, Table, TableState},
};

use crate::storage::Drive;
use crate::{
    Arch,
    storage::{Disk, DiskBuilder, Media},
};

#[derive(Debug, Clone)]
pub(super) struct StorageEdit {
    drive_state: TableState,
    new_drive: Option<DiskBuilder>,
    added_disks: Vec<Disk>,
    deleted_drive_paths: Vec<PathBuf>,
    drives: Vec<Drive>,
}

impl StorageEdit {
    pub fn new(drives: Vec<Drive>) -> Self {
        let drive_state = if drives.is_empty() {
            TableState::default()
        } else {
            TableState::default().with_selected(Some(0))
        };

        Self {
            drive_state,
            new_drive: None,
            added_disks: Vec::new(),
            deleted_drive_paths: Vec::new(),
            drives,
        }
    }

    pub fn new_drive_popup(&self) -> bool {
        self.new_drive.is_some()
    }

    pub fn deleted_drive_paths(&self) -> Vec<PathBuf> {
        self.deleted_drive_paths.clone()
    }

    pub fn added_disks(&self) -> Vec<Disk> {
        self.added_disks.clone()
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent, arch: Arch) {
        if let Some(new_drive) = &mut self.new_drive {
            match key_event.code {
                KeyCode::Esc => {
                    self.new_drive = None;
                }
                KeyCode::Enter => {
                    if new_drive.validate() {
                        let drive = new_drive.build();
                        self.added_disks.push(drive);
                        self.new_drive = None;

                        if self.drive_state.selected().is_none() {
                            self.drive_state.select(Some(0));
                        }
                    }
                }

                _ => {
                    new_drive.handle_key_events(key_event, arch);
                }
            }

            return;
        }
        match key_event.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(index) = self.drive_state.selected() {
                    self.drive_state.select(Some(
                        index
                            .saturating_add(1)
                            .min(self.drives.len() + self.added_disks.len() - 1),
                    ));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(index) = self.drive_state.selected() {
                    self.drive_state.select(Some(index.saturating_sub(1)));
                }
            }
            KeyCode::Char('d') => {
                if let Some(index) = self.drive_state.selected() {
                    if index < self.drives.len() {
                        let drive = &self.drives[index];
                        self.deleted_drive_paths.push(drive.path.clone());
                    } else {
                        let index = index.saturating_sub(self.drives.len());
                        self.added_disks.remove(index);
                    }
                }
            }
            KeyCode::Char('u') => {
                if let Some(index) = self.drive_state.selected()
                    && let Some(drive) = self.drives.get(index)
                {
                    self.deleted_drive_paths.retain(|path| &drive.path != path);
                }
            }
            KeyCode::Char('n') => {
                self.new_drive = Some(DiskBuilder::new());
            }
            _ => {}
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let widths = [
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(15),
        ];

        let vm_drives = self.drives.iter().map(|drive| {
            let to_delete = self.deleted_drive_paths.contains(&drive.path);
            Row::new(vec![
                {
                    if to_delete {
                        "Del".to_string()
                    } else {
                        String::new()
                    }
                },
                {
                    match drive.media {
                        Media::Disk => "Disk    ".to_string(),
                        Media::CdRom => "Cdrom   ".to_string(),
                    }
                },
                drive.format.to_string(),
                {
                    if let Some(size) = drive.size {
                        match size {
                            0..1_000_000 => {
                                format!("{:3}KiB", size / 1024)
                            }
                            1_000_000..1_000_000_000 => {
                                format!("{:3}MiB", size / (1024 * 1024))
                            }
                            _ => {
                                format!("{:3}GiB", size / (1024 * 1024 * 1024))
                            }
                        }
                    } else {
                        "-".to_string()
                    }
                },
                drive.interface.to_string(),
                drive
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            ])
            .style(if to_delete {
                Style::new().red()
            } else {
                Style::default()
            })
        });

        let new_drives = self.added_disks.iter().map(|drive| {
            Row::new(vec![
                "New".to_string(),
                "Disk    ".to_string(),
                drive.format.to_string(),
                format!("{} GiB", drive.size),
                drive.interface.to_string(),
                "-".to_string(),
            ])
            .green()
        });

        let mut drives: Vec<Row> = Vec::new();
        drives.extend(vm_drives);
        drives.extend(new_drives);

        let disks = Table::new(drives, widths)
            .header(
                Row::new(vec!["", "Type", "Format", "Size", "Interface", "File Name"])
                    .style(Style::new().bold())
                    .bottom_margin(1),
            )
            .flex(ratatui::layout::Flex::SpaceBetween)
            .row_highlight_style(Style::new().on_dark_gray())
            .column_spacing(1);

        frame.render_stateful_widget(disks, area, &mut self.drive_state);

        if let Some(new_drive) = &self.new_drive {
            new_drive.render(frame);
        }
    }
}
