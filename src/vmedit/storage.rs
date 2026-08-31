use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use anyhow::anyhow;
use crossterm::event::{KeyCode, KeyEvent};

use ratatui::text::Line;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, Row, Table, TableState},
};
use regex::Regex;
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::storage::Drive;
use crate::{
    Arch,
    storage::{Disk, DiskBuilder, Media},
};

#[derive(Debug, Clone)]
pub(super) struct StorageEdit {
    drive_state: TableState,
    new_drive: Option<DiskBuilder>,
    drive_resize: Option<DriveResize>,
    added_disks: Vec<Disk>,
    resized_drives: HashMap<PathBuf, String>,
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
            resized_drives: HashMap::new(),
            drive_resize: None,
            drives,
        }
    }

    pub fn new_drive_popup(&self) -> bool {
        self.new_drive.is_some() | self.drive_resize.is_some()
    }

    pub fn deleted_drive_paths(&self) -> Vec<PathBuf> {
        self.deleted_drive_paths.clone()
    }

    pub fn added_disks(&self) -> Vec<Disk> {
        self.added_disks.clone()
    }

    pub fn resized_drives(&self) -> HashMap<PathBuf, String> {
        self.resized_drives.clone()
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

        if let Some(resize) = &mut self.drive_resize {
            match key_event.code {
                KeyCode::Esc => {
                    self.drive_resize = None;
                }

                KeyCode::Enter => {
                    if let Some(new_size) = resize.apply() {
                        if let Some(index) = self.drive_state.selected()
                            && let Some(drive) = self.drives.get(index)
                        {
                            self.resized_drives.insert(drive.path.clone(), new_size);
                        }
                        self.drive_resize = None;
                    }
                }

                _ => {
                    resize.handle_key_events(key_event);
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
                        self.resized_drives.retain(|path, _| &drive.path != path);
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
                    self.resized_drives.retain(|path, _| &drive.path != path);
                }
            }
            KeyCode::Char('n') => {
                self.new_drive = Some(DiskBuilder::new());
            }

            KeyCode::Char('r') => {
                if let Some(index) = self.drive_state.selected()
                    && let Some(drive) = self.drives.get(index)
                    && !self.deleted_drive_paths.contains(&drive.path)
                    && !self.resized_drives.contains_key(&drive.path)
                {
                    self.drive_resize = Some(DriveResize::new(drive));
                }
            }
            _ => {}
        }
    }

    fn new_size_to_u16(current_size: u64, new_size: &str) -> Result<u64> {
        let re = Regex::new(r"([\+-]?)(\d+)([KMG])").unwrap();

        let caps = re.captures(new_size).unwrap();

        let sign = caps.get(1).unwrap().as_str();
        let value = caps.get(2).unwrap().as_str();
        let value = value.parse::<u64>().unwrap();
        let unit = caps.get(3).unwrap().as_str();

        match unit {
            "G" => {
                let value = value * 1024 * 1024 * 1024;
                match sign {
                    "+" => Ok(current_size + value),
                    "-" => {
                        if current_size < value {
                            Err(anyhow!("The end size can not be negative"))
                        } else {
                            Ok(current_size - value)
                        }
                    }
                    "" => Ok(value),
                    _ => unreachable!(),
                }
            }
            "M" => {
                let value = value * 1024 * 1024;
                match sign {
                    "+" => Ok(current_size + value),
                    "-" => {
                        if current_size < value {
                            Err(anyhow!("The end size can not be negative"))
                        } else {
                            Ok(value - current_size)
                        }
                    }
                    "" => Ok(value),
                    _ => unreachable!(),
                }
            }
            "K" => {
                let value = value * 1024;
                match sign {
                    "+" => Ok(current_size + value),
                    "-" => {
                        if current_size < value {
                            Err(anyhow!("The end size can not be negative"))
                        } else {
                            Ok(value - current_size)
                        }
                    }
                    "" => Ok(value),
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let widths = [
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(16),
            Constraint::Length(10),
            Constraint::Length(15),
        ];

        let vm_drives = self.drives.iter().map(|drive| {
            let to_delete = self.deleted_drive_paths.contains(&drive.path);
            let resized = self.resized_drives.contains_key(&drive.path);
            Row::new(vec![
                Line::from({
                    if to_delete {
                        "Delete".to_string()
                    } else if resized {
                        "Resize".to_string()
                    } else {
                        String::new()
                    }
                }),
                Line::from({
                    match drive.media {
                        Media::Disk => "Disk    ".to_string(),
                        Media::CdRom => "Cdrom   ".to_string(),
                    }
                }),
                Line::from(drive.format.to_string()),
                Line::from({
                    if let Some(size) = drive.size {
                        if resized {
                            let new_size = self.resized_drives.get(&drive.path).unwrap();
                            let new_size = Self::new_size_to_u16(size, new_size).unwrap();
                            let new_size = Drive::format_size(new_size);
                            let size = Drive::format_size(size);
                            format!("{size} -> {new_size}")
                        } else {
                            Drive::format_size(size)
                        }
                    } else {
                        "-".to_string()
                    }
                })
                .centered(),
                Line::from(drive.interface.to_string()),
                Line::from(
                    drive
                        .path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                ),
            ])
            .style(if to_delete {
                Style::new().red()
            } else if resized {
                Style::new().yellow()
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
                Row::new(vec![
                    Line::from(""),
                    Line::from("Type"),
                    Line::from("Format"),
                    Line::from("Size").centered(),
                    Line::from("Interface"),
                    Line::from("File Name"),
                ])
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

        if let Some(resize) = &self.drive_resize {
            resize.render(frame);
        }
    }
    pub fn help(&self) -> Vec<Line<'static>> {
        if self.new_drive.is_some() {
            vec![Line::from(vec![
                Span::from("k,↑").bold(),
                Span::from("  Up"),
                Span::from(" | "),
                Span::from("j,↓").bold(),
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
        } else {
            vec![Line::from(vec![
                Span::from("Esc").bold(),
                Span::from(" Cancel"),
                Span::from(" | "),
                Span::from("Enter").bold(),
                Span::from(" Confirm"),
            ])]
        }
    }
}

// Resize
#[derive(Debug, Clone)]
pub struct DriveResize {
    drive: Drive,
    new_size: UserInputField,
}

#[derive(Debug, Clone, Default)]
struct UserInputField {
    field: Input,
    error: Option<String>,
}

impl DriveResize {
    pub fn new(drive: &Drive) -> Self {
        Self {
            drive: drive.clone(),
            new_size: UserInputField {
                field: Input::default(),
                error: None,
            },
        }
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        self.new_size
            .field
            .handle_event(&crossterm::event::Event::Key(key_event));
    }

    fn validate(&mut self) -> bool {
        let mut valid = true;

        let input = self.new_size.field.value();

        if input.is_empty() {
            self.new_size.error = Some("Field required".into());
            valid = false;
        } else {
            let regex = Regex::new(r"[\+-]?\d+[KMG]").unwrap();
            if !regex.is_match(input) {
                self.new_size.error = Some("Invalid input".into());
                valid = false;
            }
        }

        valid
    }

    fn apply(&mut self) -> Option<String> {
        if self.validate() {
            Some(self.new_size.field.value().to_string())
        } else {
            None
        }
    }
    pub fn render(&self, frame: &mut Frame) {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(10),
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
                .title(" Resize 󰩨  ")
                .border_type(BorderType::Thick)
                .title_alignment(ratatui::layout::HorizontalAlignment::Center)
                .border_style(Style::default().yellow()),
            area,
        );

        let area = area.inner(Margin {
            horizontal: 8,
            vertical: 2,
        });

        let rows = [
            Row::new(vec![
                Span::from("Current Size").bold(),
                Span::from({
                    if let Some(size) = self.drive.size {
                        Drive::format_size(size)
                    } else {
                        "-".to_string()
                    }
                }),
            ]),
            Row::new(vec!["", ""]),
            Row::new(vec![
                Span::from("New Size").bold(),
                Span::from({
                    let original_length = self.new_size.field.to_string().len();
                    let target_length = 30_usize;

                    self.new_size
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
            Row::new(vec![
                Span::from(""),
                Span::from(self.new_size.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];
        let widths = [Constraint::Length(14), Constraint::Length(30)];
        let table = Table::new(rows, widths);
        frame.render_widget(table, area);

        if self.new_size.field.visual_cursor() < 30 {
            let x = area.x + self.new_size.field.visual_cursor() as u16 + 15;
            let y = area.y + 2;
            frame.set_cursor_position((x, y));
        }
    }
}
