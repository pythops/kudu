use crossterm::event::{
    KeyCode::{self},
    KeyEvent,
};
use rustix::path::Arg;
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Command};

use anyhow::Result;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Drive {
    pub path: PathBuf,
    pub interface: Interface,
    pub media: Media,
    pub format: Format,
    pub read_only: bool,
    pub unit: Option<u16>,
    pub size: Option<u64>,
}

impl Drive {
    pub fn size(&self) -> Result<u64> {
        let output = Command::new("qemu-img")
            .arg("info")
            .arg("--output")
            .arg("json")
            .arg(self.path.clone())
            .output()?;

        let output = String::from_utf8(output.stdout)?;
        let output: Value = serde_json::from_str(&output)?;
        Ok(output["virtual-size"].as_u64().unwrap() / (1024 * 1024 * 1024))
    }

    pub fn to_qemu_arg(&self) -> String {
        let mut args = Vec::new();

        args.push(format!("file={}", self.path.to_string_lossy()));

        args.push(format!("if={}", self.interface));

        args.push(format!("format={}", self.format));

        if self.read_only {
            args.push("readonly=on".to_string());
        }

        if self.media == Media::CdRom {
            args.push("media=cdrom".to_string());
        }

        if let Some(unit) = self.unit {
            args.push(format!("unit={}", unit));
        }

        args.join(",").to_string()
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Interface {
    #[default]
    Virtio,
    Ide,
    Pflash,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, strum::Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
pub enum Media {
    #[default]
    Disk,
    CdRom,
}

#[derive(Debug, Clone, Default)]
pub struct DiskBuilder {
    section: Section,
    pub format: Format,
    size: UserInputField,
}

#[derive(Debug, Clone, Default)]
struct UserInputField {
    field: Input,
    error: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Disk {
    pub size: u64,
    pub format: Format,
}

impl Disk {
    pub fn create(&self, path: &Path) -> Result<()> {
        Command::new("qemu-img")
            .arg("create")
            .arg("-f")
            .arg(self.format.to_string().to_lowercase())
            .arg(path)
            .arg(format!("{}G", self.size))
            .output()?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, strum::Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
pub enum Format {
    #[default]
    Qcow2,
    Raw,
}

#[derive(Debug, Default, Clone, PartialEq)]
enum Section {
    #[default]
    Size,
    Format,
}

impl DiskBuilder {
    pub fn new() -> Self {
        DiskBuilder::default()
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Down | KeyCode::Up | KeyCode::Char('j') | KeyCode::Char('k') => {
                match self.section {
                    Section::Size => self.section = Section::Format,
                    Section::Format => self.section = Section::Size,
                }
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l')
                if self.section == Section::Format =>
            {
                match self.format {
                    Format::Qcow2 => self.format = Format::Raw,
                    Format::Raw => self.format = Format::Qcow2,
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

    pub fn build(&self) -> Disk {
        let size = self.size.field.value().parse::<u64>().unwrap();
        let format = self.format;

        Disk { size, format }
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
                Span::from(" ".repeat(8)),
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
                Span::from(" ".repeat(16)),
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
