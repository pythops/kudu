mod hardware;
mod network;
mod overview;
mod storage;

use anyhow::Result;
use std::{mem::discriminant, path::PathBuf, sync::mpsc::Sender};

use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, Padding},
};

use crate::{
    Arch, BootOption,
    confirmation::cancel::CancelConfirmation,
    distro::LinuxDistro::{self},
    event::Event,
    network::NetworkBackend,
    storage::Disk,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Section {
    Overview,
    Hardware,
    Storage,
    Network,
    Summary,
}

#[derive(Debug, Clone)]
pub struct VMBuilder {
    pub focused_section: Section,
    pub overview: overview::Overview,
    pub hardware: hardware::Hardware,
    pub storage: storage::Storage,
    pub network: network::Network,
    pub cancel_confirmation: Option<CancelConfirmation>,
}

impl Default for VMBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct VMBuildData {
    pub boot_option: BootOption,
    pub arch: Arch,
    pub name: String,
    pub cloudinit: Option<PathBuf>,
    pub boot_file: Option<PathBuf>,
    pub os: Option<LinuxDistro>,
    pub vcpu: u16,
    pub memory: u32,
    pub network_backend: NetworkBackend,
    pub enable_uefi: bool,
    pub disks: Vec<Disk>,
}

impl VMBuilder {
    pub fn new() -> VMBuilder {
        Self {
            focused_section: Section::Overview,
            overview: overview::Overview::new(),
            hardware: hardware::Hardware::new(),
            storage: storage::Storage::new(),
            network: network::Network::new(),
            cancel_confirmation: None,
        }
    }

    pub fn build(&self) -> VMBuildData {
        VMBuildData {
            boot_option: self.overview.boot_option(),
            name: self.overview.name(),
            cloudinit: self.overview.cloudinit(),
            boot_file: self.overview.boot_file(),
            os: self.overview.os(),
            vcpu: self.hardware.vcpu(),
            memory: self.hardware.memory(),
            arch: self.hardware.arch(),
            enable_uefi: self.hardware.enable_uefi(),
            network_backend: self.network.backend(),
            disks: self.storage.disks(),
        }
    }

    fn validate_overview_section(&mut self) -> bool {
        self.overview.validate()
    }

    fn validate_harware_section(&mut self) -> bool {
        self.hardware.validate()
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent, sender: Sender<Event>) -> Result<()> {
        if self.cancel_confirmation.is_some() && key_event.code == KeyCode::Esc {
            self.cancel_confirmation = None;
            return Ok(());
        }

        if let Some(confirmation) = &mut self.cancel_confirmation {
            confirmation.handle_key_events(key_event, sender)?;
            return Ok(());
        }

        if self.storage.new_disk_popup() {
            self.storage.handle_key_events(key_event);
            return Ok(());
        }

        if key_event.code == KeyCode::Esc {
            self.cancel_confirmation = Some(CancelConfirmation::default());
            return Ok(());
        }

        match key_event.code {
            KeyCode::Tab => match self.focused_section {
                Section::Overview => {
                    if self.validate_overview_section() {
                        self.focused_section = Section::Hardware;
                    }
                }
                Section::Hardware => {
                    if self.validate_harware_section() {
                        self.focused_section = Section::Storage;
                    }
                }
                Section::Storage => self.focused_section = Section::Network,
                Section::Network => self.focused_section = Section::Summary,
                Section::Summary => {
                    self.focused_section = Section::Overview;
                }
            },
            KeyCode::BackTab => match self.focused_section {
                Section::Overview => {
                    if self.validate_overview_section() {
                        self.focused_section = Section::Summary;
                    }
                }
                Section::Hardware => {
                    if self.validate_harware_section() {
                        self.focused_section = Section::Overview;
                    }
                }
                Section::Storage => self.focused_section = Section::Hardware,
                Section::Network => self.focused_section = Section::Storage,
                Section::Summary => self.focused_section = Section::Network,
            },
            _ => match &self.focused_section {
                Section::Overview => {
                    self.overview.handle_key_events(key_event);
                }
                Section::Hardware => {
                    self.hardware
                        .handle_key_events(key_event, self.overview.os());
                }
                Section::Storage => {
                    self.storage.handle_key_events(key_event);
                }
                Section::Network => {}
                Section::Summary if key_event.code == KeyCode::Enter => {
                    let vm_build_data = self.build();
                    sender.send(Event::VMCreated(vm_build_data))?;
                }
                _ => {}
            },
        }

        Ok(())
    }

    fn title_span(&self, section: Section) -> Span<'_> {
        let is_focused = discriminant(&self.focused_section) == discriminant(&section);
        match section {
            Section::Overview => {
                if is_focused {
                    Span::styled(
                        "   Overview     ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from("   Overview     ").fg(Color::DarkGray)
                }
            }
            Section::Hardware => {
                if is_focused {
                    Span::styled(
                        "  Hardware    ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from("  Hardware    ").fg(Color::DarkGray)
                }
            }
            Section::Storage => {
                if is_focused {
                    Span::styled(
                        "   Storage 󱛟   ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from("   Storage 󱛟   ").fg(Color::DarkGray)
                }
            }
            Section::Network => {
                if is_focused {
                    Span::styled(
                        "  Network 󰛳    ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from("  Network 󰛳    ").fg(Color::DarkGray)
                }
            }
            Section::Summary => {
                if is_focused {
                    Span::styled(
                        "  Summary  󱇗   ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from("  Summary  󱇗   ").fg(Color::DarkGray)
                }
            }
        }
    }
    fn render_header(&self, frame: &mut Frame, block: Rect) {
        frame.render_widget(
            Block::default()
                .title({
                    Line::from(vec![
                        self.title_span(Section::Overview),
                        self.title_span(Section::Hardware),
                        self.title_span(Section::Storage),
                        self.title_span(Section::Network),
                        self.title_span(Section::Summary),
                    ])
                })
                .title_alignment(Alignment::Center)
                .padding(Padding::top(1)),
            block,
        );
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(35),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(frame.area())[1];

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(100),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(area)[1];

        frame.render_widget(Clear, area);

        frame.render_widget(
            Block::new()
                .title(" New VM 󰏖  ")
                .title_alignment(ratatui::layout::HorizontalAlignment::Center)
                .borders(Borders::all())
                .border_type(
                    if self.cancel_confirmation.is_some() | self.storage.new_disk_popup() {
                        BorderType::default()
                    } else {
                        BorderType::Thick
                    },
                )
                .border_style(Style::default().yellow()),
            area,
        );

        let area = area.inner(Margin {
            horizontal: 5,
            vertical: 2,
        });

        let (section_block, area) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Fill(1)])
                .flex(ratatui::layout::Flex::SpaceBetween)
                .split(area);

            (chunks[0], chunks[1])
        };

        self.render_header(frame, section_block);

        match &self.focused_section {
            Section::Overview => {
                self.overview
                    .render(frame, area, self.cancel_confirmation.is_some());
            }
            Section::Hardware => {
                self.hardware.render(
                    frame,
                    area,
                    self.overview.os(),
                    self.cancel_confirmation.is_some(),
                );
            }

            Section::Storage => {
                self.storage.render(frame, area);
            }

            Section::Network => {
                self.network.render(frame, area);
            }

            Section::Summary => {
                let (summary_block, create_block) = {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Fill(1), Constraint::Length(3)])
                        .margin(1)
                        .split(area);

                    (chunks[0], chunks[1])
                };
                let mut items = Vec::new();

                items.extend(self.overview.summary());
                items.extend(self.hardware.summary());
                items.extend(self.storage.summary());
                items.extend(self.network.summary());

                let list_width = items.iter().map(|item| item.width()).max().unwrap() as u16;
                let list = List::new(items);
                let create = Text::from(vec![Line::from(""), Line::from("CREATE"), Line::from("")])
                    .centered()
                    .black()
                    .on_yellow()
                    .bold();

                let summary_block = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Fill(1),
                        Constraint::Length(list_width),
                        Constraint::Fill(1),
                    ])
                    .flex(ratatui::layout::Flex::Center)
                    .split(summary_block)[1];

                frame.render_widget(
                    list,
                    summary_block.inner(Margin {
                        horizontal: 0,
                        vertical: 1,
                    }),
                );

                let create_block = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Fill(1),
                        Constraint::Length(20),
                        Constraint::Fill(1),
                    ])
                    .flex(ratatui::layout::Flex::SpaceBetween)
                    .split(create_block)[1];

                frame.render_widget(create, create_block);
            }
        }

        if let Some(confirmation) = &self.cancel_confirmation {
            confirmation.render(frame);
        }
    }
}
