use serde::{Deserialize, Serialize};

use crossterm::event::{KeyCode, KeyEvent};
use rustix::path::Arg;

use std::path::PathBuf;

use tui_input::{Input, backend::crossterm::EventHandler};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear},
};

use crate::fs::Section::ReadOnly;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filesystem {
    pub driver: FsDriver,
    pub source_path: PathBuf,
    pub mount_tag: String,
    pub readonly: bool,
}

impl Filesystem {
    pub fn to_qemu_arg(&self) -> Vec<String> {
        match self.driver {
            FsDriver::Virtio9pPci => {
                let readonly = if self.readonly { "on" } else { "off" };
                vec![
                    "-virtfs".to_string(),
                    format!(
                        "local,path={},mount_tag={},security_model=none,readonly={}",
                        self.source_path.to_string_lossy(),
                        self.mount_tag,
                        readonly
                    ),
                ]
            }
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, strum::Display, Default, Serialize, Deserialize)]
pub enum FsDriver {
    #[default]
    #[strum(to_string = "9pfs")]
    Virtio9pPci,
}

// NewFs

#[derive(Debug, Clone, Default, PartialEq)]
enum Section {
    #[default]
    SourcePath,
    MountTag,
    ReadOnly,
}

#[derive(Debug, Clone, Default)]
pub struct NewFs {
    section: Section,
    driver: FsDriver,
    source_path: UserInputField,
    mount_tag: UserInputField,
    readonly: bool,
}

#[derive(Debug, Clone, Default)]
struct UserInputField {
    field: Input,
    error: Option<String>,
}

impl NewFs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Down => match self.section {
                Section::SourcePath => {
                    self.section = Section::MountTag;
                }
                Section::MountTag => {
                    self.section = Section::ReadOnly;
                }
                ReadOnly => {
                    self.section = Section::SourcePath;
                }
            },
            KeyCode::Up => match self.section {
                Section::SourcePath => {
                    self.section = Section::ReadOnly;
                }
                Section::MountTag => {
                    self.section = Section::SourcePath;
                }
                ReadOnly => {
                    self.section = Section::MountTag;
                }
            },
            _ => match self.section {
                Section::SourcePath => {
                    self.source_path
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
                Section::MountTag => {
                    self.mount_tag
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
                Section::ReadOnly => match key_event.code {
                    KeyCode::Left | KeyCode::Char('l') | KeyCode::Right | KeyCode::Char('h') => {
                        self.readonly = !self.readonly;
                    }
                    _ => {}
                },
            },
        }
    }

    pub fn validate(&mut self) -> bool {
        let mut valid = true;

        let source_path = self.source_path.field.value();
        let mount_tag = self.mount_tag.field.value();

        if source_path.is_empty() {
            self.source_path.error = Some("Field required".into());
            valid = false;
        } else {
            if !PathBuf::from(source_path).exists() {
                self.source_path.error = Some("Invalid Path".into());
                valid = false;
            }
        }

        if mount_tag.is_empty() {
            self.mount_tag.error = Some("Field required".into());
            valid = false;
        } else {
            if mount_tag.chars().any(|c| !c.is_alphanumeric()) {
                self.source_path.error = Some("Mount tag should be alphanumeric".into());
                valid = false;
            }
        }

        valid
    }

    pub fn build(&self) -> Filesystem {
        Filesystem {
            driver: FsDriver::Virtio9pPci,
            source_path: PathBuf::from(self.source_path.field.value().to_string()),
            mount_tag: self.mount_tag.field.value().to_string(),
            readonly: self.readonly,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(19),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(frame.area())[1];

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(80),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(area)[1];

        frame.render_widget(Clear, area);

        frame.render_widget(
            Block::new()
                .borders(Borders::all())
                .title(" Filesystem   ")
                .border_type(BorderType::Thick)
                .title_alignment(ratatui::layout::HorizontalAlignment::Center)
                .border_style(Style::default().yellow()),
            area,
        );

        let area = area.inner(Margin {
            horizontal: 4,
            vertical: 2,
        });

        let (driver_block, source_block, mount_tag_block, readonly_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Length(2),
                    Constraint::Length(2),
                    Constraint::Length(2),
                ])
                .margin(1)
                .flex(ratatui::layout::Flex::SpaceBetween)
                .split(area);

            (chunks[0], chunks[1], chunks[2], chunks[3])
        };

        let driver = Text::from(Line::from(vec![
            Span::from("  Driver").bold(),
            Span::from(" ".repeat(7)),
            Span::from(self.driver.to_string()),
        ]));

        let source_path = Text::from(vec![
            Line::from(vec![
                {
                    if self.section == Section::SourcePath {
                        Span::from("> Source Path").bold()
                    } else {
                        Span::from("  Source Path")
                    }
                },
                Span::from(" ".repeat(2)),
                Span::from({
                    let original_length = self.source_path.field.to_string().len();
                    let target_length = 60_usize;

                    self.source_path
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
            Line::from(vec![
                Span::from(" ".repeat(15)),
                Span::from(self.source_path.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ]);

        let mount_tag = Text::from(vec![
            Line::from(vec![
                {
                    if self.section == Section::MountTag {
                        Span::from("> Mount Tag").bold()
                    } else {
                        Span::from("  Mount Tag")
                    }
                },
                Span::from(" ".repeat(4)),
                Span::from({
                    let original_length = self.mount_tag.field.to_string().len();
                    let target_length = 60_usize;

                    self.mount_tag
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
            Line::from(vec![
                Span::from(" ".repeat(15)),
                Span::from(self.mount_tag.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ]);

        let readonly = Text::from(Line::from(vec![
            {
                if self.section == Section::ReadOnly {
                    Span::from("> Read only").bold()
                } else {
                    Span::from("  Read only")
                }
            },
            Span::from(" ".repeat(4)),
            Span::from({
                if self.readonly {
                    "[x] Yes       [ ] No"
                } else {
                    "[ ] Yes       [x] No"
                }
            }),
        ]));

        frame.render_widget(driver, driver_block);
        frame.render_widget(source_path, source_block);
        frame.render_widget(mount_tag, mount_tag_block);
        frame.render_widget(readonly, readonly_block);

        match self.section {
            Section::SourcePath if self.source_path.field.visual_cursor() < 56 => {
                let x = area.x + self.source_path.field.visual_cursor() as u16 + 16;
                let y = area.y + 4;
                frame.set_cursor_position((x, y));
            }
            Section::MountTag if self.mount_tag.field.visual_cursor() < 56 => {
                let x = area.x + self.mount_tag.field.visual_cursor() as u16 + 16;
                let y = area.y + 7;
                frame.set_cursor_position((x, y));
            }
            _ => {}
        }
    }
}
