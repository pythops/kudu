use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{ListItem, Row, Table},
};

use crossterm::event::{
    KeyCode::{self},
    KeyEvent,
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::graphics::{Display, Graphics, GraphicsDevice, VirtioConfig};

#[derive(Debug, Clone, PartialEq, Default)]
enum Section {
    #[default]
    Device,
    Display,
    Memory,
    Enable3d,
    Venus,
}

#[derive(Debug, Clone, Default)]
struct UserInputField {
    field: Input,
    error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, strum::Display)]
enum Device {
    #[default]
    Std,
    Qxl,
    Virtio,
    None,
}

#[derive(Debug, Clone, Default)]
pub struct GraphicsBuilder {
    section: Section,
    device: Device,
    display: Display,
    memory: UserInputField,
    enable_3d: bool,
    venus: bool,
}

impl GraphicsBuilder {
    pub fn new() -> Self {
        Self {
            section: Section::default(),
            device: Device::default(),
            display: Display::default(),
            memory: UserInputField {
                field: "16".into(),
                error: None,
            },
            enable_3d: false,
            venus: false,
        }
    }

    pub fn build(&self) -> Graphics {
        match self.device {
            Device::None => Graphics {
                device: GraphicsDevice::None,
                display: Display::None,
            },

            Device::Std => {
                let memory = self.memory.field.value().parse::<u16>().unwrap();
                Graphics {
                    device: GraphicsDevice::Std { vgamem: memory },
                    display: self.display,
                }
            }
            Device::Qxl => {
                let memory = self.memory.field.value().parse::<u16>().unwrap();
                Graphics {
                    device: GraphicsDevice::Qxl { vgamem: memory },
                    display: self.display,
                }
            }
            Device::Virtio => Graphics {
                device: GraphicsDevice::Virtio(VirtioConfig {
                    enable_3d: self.enable_3d,
                    venus: self.venus,
                }),
                display: self.display,
            },
        }
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Down | KeyCode::Char('j') => match self.section {
                Section::Device => match self.device {
                    Device::None => {}
                    _ => {
                        self.section = Section::Display;
                    }
                },
                Section::Display => match self.device {
                    Device::Std | Device::Qxl => {
                        self.section = Section::Memory;
                    }
                    Device::Virtio => {
                        self.section = Section::Enable3d;
                    }
                    Device::None => {
                        unreachable!()
                    }
                },
                Section::Memory => match self.device {
                    Device::Std | Device::Qxl => {
                        self.section = Section::Device;
                    }
                    _ => {
                        unreachable!()
                    }
                },
                Section::Enable3d => {
                    self.section = Section::Venus;
                }
                Section::Venus => {
                    self.section = Section::Device;
                }
            },
            KeyCode::Up | KeyCode::Char('k') => match self.section {
                Section::Device => {}
                Section::Display => {}
                Section::Memory => {}
                Section::Enable3d => {}
                Section::Venus => {}
            },
            _ => match self.section {
                Section::Device => match key_event.code {
                    KeyCode::Char('l') | KeyCode::Right => match self.device {
                        Device::Std => {
                            self.device = Device::Qxl;
                            self.memory = UserInputField {
                                field: Input::from("16"),
                                error: None,
                            };
                        }
                        Device::Qxl => {
                            self.device = Device::Virtio;
                        }
                        Device::Virtio => {
                            self.device = Device::None;
                        }
                        Device::None => {
                            self.device = Device::Std;
                            self.memory = UserInputField {
                                field: Input::from("16"),
                                error: None,
                            };
                        }
                    },
                    KeyCode::Char('h') | KeyCode::Left => match self.device {
                        Device::Std => {
                            self.device = Device::None;
                        }
                        Device::Qxl => {
                            self.device = Device::Std;
                            self.memory = UserInputField {
                                field: Input::from("16"),
                                error: None,
                            };
                        }
                        Device::Virtio => {
                            self.device = Device::Qxl;
                            self.memory = UserInputField {
                                field: Input::from("16"),
                                error: None,
                            };
                        }
                        Device::None => {
                            self.device = Device::Virtio;
                        }
                    },
                    _ => {}
                },
                Section::Display if self.device != Device::None => match key_event.code {
                    KeyCode::Char('l') | KeyCode::Right => match self.display {
                        Display::None => {
                            self.display = Display::Gtk;
                        }
                        Display::Gtk => {
                            self.display = Display::Sdl;
                        }
                        Display::Sdl => {
                            self.display = Display::EglHeadless;
                        }
                        Display::EglHeadless => {
                            self.display = Display::None;
                        }
                    },
                    KeyCode::Char('h') | KeyCode::Left => {}
                    _ => {}
                },
                Section::Memory => {
                    self.memory
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
                Section::Enable3d => match key_event.code {
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
                        self.enable_3d = !self.enable_3d;

                        if self.enable_3d && self.display == Display::None {
                            self.display = Display::EglHeadless;
                        }
                    }
                    _ => {}
                },
                Section::Venus => match key_event.code {
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
                        self.venus = !self.venus;

                        if self.venus && self.display == Display::None {
                            self.display = Display::EglHeadless;
                        }
                    }
                    _ => {}
                },
                _ => {}
            },
        }
    }

    pub fn validate(&mut self) -> bool {
        let mut valid = true;

        self.memory.error = None;

        match self.device {
            Device::Std | Device::Qxl => {
                let memory = self.memory.field.value();

                if memory.is_empty() {
                    self.memory.error = Some("Field required".into());
                    valid = false;
                } else {
                    match self.memory.field.value().parse::<u32>() {
                        Ok(v) => {
                            if v == 0 {
                                self.memory.error = Some("Memory value can not be 0".into());
                                valid = false;
                            }
                        }
                        Err(_) => {
                            self.memory.error = Some("Memory value should be a number".into());
                            valid = false;
                        }
                    }
                }
            }
            _ => {}
        }

        valid
    }

    pub fn summary(&self) -> Vec<ListItem<'_>> {
        vec![ListItem::from(vec![
            Line::from(vec![
                Span::from("Graphics").bold(),
                Span::from(" ".repeat(12)),
                Span::from(format!(
                    "Card: {} -- Display: {}",
                    self.device, self.display,
                )),
            ]),
            Line::from(""),
        ])]
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let widths = [
            Constraint::Length(25),
            Constraint::Length(60),
            Constraint::Length(3),
        ];
        let mut rows = vec![
            Row::new(vec![
                {
                    if self.section == Section::Device {
                        Span::from("> Card").bold()
                    } else {
                        Span::from("  Card")
                    }
                },
                Span::from(format!("< {} >", self.device)),
            ]),
            Row::new(Line::from("")),
        ];

        if self.device != Device::None {
            rows.extend(vec![
                Row::new(vec![
                    {
                        if self.section == Section::Display {
                            Span::from("> Display").bold()
                        } else {
                            Span::from("  Display")
                        }
                    },
                    Span::from(format!("< {} >", self.display)),
                ]),
                Row::new(Line::from("")),
            ])
        }

        match self.device {
            Device::Std | Device::Qxl => {
                rows.extend([
                    Row::new(vec![
                        {
                            if self.section == Section::Memory {
                                Span::from("> Memory").bold()
                            } else {
                                Span::from("  Memory")
                            }
                        },
                        Span::from({
                            let original_length = self.memory.field.to_string().len();
                            let target_length = 65_usize;

                            self.memory
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
                        Span::from("MiB"),
                    ]),
                    Row::new(vec![
                        Span::from(""),
                        Span::from(self.memory.clone().error.unwrap_or("".into())).red(),
                    ]),
                ]);
            }
            Device::Virtio => rows.extend(vec![
                Row::new(vec![
                    {
                        if self.section == Section::Enable3d {
                            Span::from("> 3D Acceleration").bold()
                        } else {
                            Span::from("  3D Acceleration")
                        }
                    },
                    Span::from({
                        if self.enable_3d {
                            "[x] Enabled         [ ] Disabled"
                        } else {
                            "[ ] Enabled         [x] Disabled"
                        }
                    }),
                ]),
                Row::new(Line::from("")),
                Row::new(vec![
                    {
                        if self.section == Section::Venus {
                            Span::from("> Venus").bold()
                        } else {
                            Span::from("  Venus")
                        }
                    },
                    Span::from({
                        if self.venus {
                            "[x] Enabled         [ ] Disabled"
                        } else {
                            "[ ] Enabled         [x] Disabled"
                        }
                    }),
                ]),
            ]),
            _ => {}
        }

        let table = Table::new(rows, widths);

        frame.render_widget(
            table,
            area.inner(Margin {
                horizontal: 2,
                vertical: 2,
            }),
        );
    }
}
