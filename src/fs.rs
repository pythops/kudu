use serde::{Deserialize, Serialize};

use crossterm::event::KeyEvent;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filesystem {
    pub driver: FsDriver,
    pub source_path: PathBuf,
}

impl Filesystem {
    pub fn to_qemu_arg(&self) -> Vec<String> {
        match self.driver {
            FsDriver::Virtio9pPci => {
                let mut mount_tag = self.source_path.to_string_lossy().to_string();
                if mount_tag.starts_with('/') {
                    mount_tag.remove(0);
                }
                mount_tag = mount_tag.replace("/", "_");
                vec![
                    "-virtfs".to_string(),
                    format!(
                        "local,path={},mount_tag={},security_model=none",
                        self.source_path.to_string_lossy(),
                        mount_tag
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

#[derive(Debug, Clone, Default)]
pub struct NewFs {
    driver: FsDriver,
    source_path: UserInputField,
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
        self.source_path
            .field
            .handle_event(&crossterm::event::Event::Key(key_event));
    }

    pub fn validate(&mut self) -> bool {
        let mut valid = true;

        let input = self.source_path.field.value();

        if input.is_empty() {
            self.source_path.error = Some("Field required".into());
            valid = false;
        } else {
            if !PathBuf::from(input).exists() {
                self.source_path.error = Some("Invalid Path".into());
                valid = false;
            }
        }

        valid
    }

    pub fn build(&self) -> Filesystem {
        Filesystem {
            driver: FsDriver::Virtio9pPci,
            source_path: PathBuf::from(self.source_path.field.value().to_string()),
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(13),
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

        let (driver_block, source_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(2), Constraint::Length(2)])
                .margin(1)
                .flex(ratatui::layout::Flex::SpaceBetween)
                .split(area);

            (chunks[0], chunks[1])
        };

        let driver = Text::from(Line::from(vec![
            Span::from("Driver").bold(),
            Span::from(" ".repeat(9)),
            Span::from(self.driver.to_string()),
        ]));

        let source_path = Text::from(vec![
            Line::from(vec![
                Span::from("Source Path").bold(),
                Span::from(" ".repeat(4)),
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

        frame.render_widget(driver, driver_block);
        frame.render_widget(source_path, source_block);

        if self.source_path.field.visual_cursor() < 56 {
            let x = area.x + self.source_path.field.visual_cursor() as u16 + 16;
            let y = area.y + 4;
            frame.set_cursor_position((x, y));
        }
    }
}
