use chrono::Utc;
use crossterm::event::{
    KeyCode::{self},
    KeyEvent,
};
use serde::{Deserialize, Serialize};
use std::process::Command;

use anyhow::Result;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
struct UserInputField {
    field: Input,
    error: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Disk {
    pub path: PathBuf,
    pub size: u32,
    pub format: DiskFormat,
}

#[derive(Debug, Clone, Default, strum_macros::Display, Serialize, Deserialize)]
pub enum DiskFormat {
    #[default]
    Qcow2,
    Raw,
}

#[derive(Debug, Default, Clone)]
pub struct DiskBuilder {
    section: Section,
    size: UserInputField,
    format: DiskFormat,
}

#[derive(Debug, Default, Clone, PartialEq)]
enum Section {
    #[default]
    Size,
    Format,
}

impl DiskBuilder {
    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Down | KeyCode::Up => match self.section {
                Section::Size => self.section = Section::Format,
                Section::Format => self.section = Section::Size,
            },
            KeyCode::Left | KeyCode::Right if self.section == Section::Format => {
                match self.format {
                    DiskFormat::Qcow2 => self.format = DiskFormat::Raw,
                    DiskFormat::Raw => self.format = DiskFormat::Qcow2,
                }
            }

            _ => {
                self.size
                    .field
                    .handle_event(&crossterm::event::Event::Key(key_event));
            }
        }
    }

    pub fn validate(&mut self) -> bool {
        let mut valid = true;

        self.size.error = None;

        if self.size.field.value().is_empty() {
            self.size.error = Some("Field required".into());
            valid = false;
        } else {
            match self.size.field.value().parse::<u16>() {
                Ok(v) => {
                    if v == 0 {
                        self.size.error = Some("Size can not be 0".into());
                        valid = false;
                    }
                }
                Err(_) => {
                    self.size.error = Some("Size value should be a number".into());
                    valid = false;
                }
            }
        }

        valid
    }

    pub fn build(&self) -> Result<Disk> {
        let size = self.size.field.value().parse::<u32>().unwrap();
        let format = self.format.clone();
        let path = std::env::temp_dir().join(format!("kudu_disk_{}", Utc::now().timestamp()));

        Command::new("qemu-img")
            .arg("create")
            .arg("-f")
            .arg(format.to_string().to_lowercase())
            .arg(&path)
            .arg(format!("{size}G"))
            .output()?;

        Ok(Disk { path, size, format })
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(14),
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
                .title(" New Disk ")
                .border_type(BorderType::Thick)
                .title_alignment(ratatui::layout::HorizontalAlignment::Center)
                .border_style(Style::default().yellow()),
            area,
        );

        let area = area.inner(Margin {
            horizontal: 2,
            vertical: 1,
        });

        let (size_block, format_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(2),
                    Constraint::Length(3),
                ])
                .margin(2)
                .split(area);

            (chunks[0], chunks[2])
        };

        let size = vec![
            Line::from(vec![
                {
                    if self.section == Section::Size {
                        Span::from("> Size  ").bold()
                    } else {
                        Span::from("  Size  ")
                    }
                },
                Span::from(" ".repeat(6)),
                Span::from({
                    let original_length = self.size.field.to_string().len();
                    let target_length = 30;

                    self.size
                        .field
                        .to_string()
                        .chars()
                        .chain(std::iter::repeat_n(' ', target_length - original_length))
                        .collect::<String>()
                })
                .on_dark_gray(),
                Span::from(" GiB"),
            ]),
            Line::from(vec![
                Span::from(" ".repeat(14)),
                Span::from(self.size.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];

        let format = Line::from(vec![
            {
                if self.section == Section::Format {
                    Span::from("> Format  ").bold()
                } else {
                    Span::from("  Format  ")
                }
            },
            Span::from(" ".repeat(6)),
            Span::from(format!("< {} >", self.format)),
        ]);

        frame.render_widget(Text::from(size), size_block);
        frame.render_widget(Text::from(format), format_block);
    }
}
