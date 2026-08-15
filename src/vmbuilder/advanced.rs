use anyhow::Result;
use std::{mem::discriminant, sync::mpsc::Sender};

use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, Padding},
};

use crate::{
    Arch,
    distro::LinuxDistro::{ArchLinux, TempleOS},
    event::Event,
    vmbuilder::{VMBuildData, access, hardware, network, overview, port, storage},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Section {
    Overview,
    Hardware,
    Storage,
    Network,
    PortForwarding,
    RemoteAccess,
    Summary,
}

#[derive(Debug, Clone)]
pub struct Advanced {
    pub focused_section: Section,
    pub overview: overview::Overview,
    pub hardware: hardware::Hardware,
    pub storage: storage::Storage,
    pub network: network::Network,
    pub port_fowrwaring: port::PortForwaring,
    pub remote_access: access::RemoteAccessBuilder,
}

impl Default for Advanced {
    fn default() -> Self {
        Self::new()
    }
}

impl Advanced {
    pub fn new() -> Advanced {
        Self {
            focused_section: Section::Overview,
            overview: overview::Overview::new(),
            hardware: hardware::Hardware::new(),
            storage: storage::Storage::new(),
            network: network::Network::new(),
            port_fowrwaring: port::PortForwaring::new(),
            remote_access: access::RemoteAccessBuilder::new(),
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
            port_mappings: self.port_fowrwaring.port_mappings(),
            remote_access: self.remote_access.access(),
        }
    }

    fn validate_overview_section(&mut self) -> bool {
        self.overview.validate()
    }

    fn validate_harware_section(&mut self) -> bool {
        self.hardware.validate()
    }

    fn validate_remote_access(&mut self) -> bool {
        self.remote_access.validate()
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent, sender: Sender<Event>) -> Result<()> {
        if self.storage.new_disk_popup() {
            self.storage
                .handle_key_events(key_event, self.hardware.arch());
            return Ok(());
        }

        if self.port_fowrwaring.new_mapping_popup() {
            self.port_fowrwaring.handle_key_events(key_event);
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
                Section::Network => self.focused_section = Section::PortForwarding,
                Section::PortForwarding => self.focused_section = Section::RemoteAccess,
                Section::RemoteAccess => {
                    if self.validate_remote_access() {
                        self.focused_section = Section::Summary
                    }
                }
                Section::Summary => {
                    self.focused_section = Section::Overview;
                }
            },
            KeyCode::BackTab => match self.focused_section {
                Section::Overview => {
                    if self.validate_overview_section() {
                        if self.overview.os() == Some(TempleOS) {
                            self.hardware.set_arch(Arch::X86_64);
                            self.hardware.set_uefi(false);
                        }
                        if self.overview.os() == Some(ArchLinux) {
                            self.hardware.set_arch(Arch::X86_64);
                        }
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
                Section::PortForwarding => self.focused_section = Section::Network,
                Section::RemoteAccess => {
                    if self.validate_remote_access() {
                        self.focused_section = Section::PortForwarding
                    }
                }
                Section::Summary => self.focused_section = Section::RemoteAccess,
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
                    self.storage
                        .handle_key_events(key_event, self.hardware.arch());
                }
                Section::Network => {}
                Section::PortForwarding => {
                    self.port_fowrwaring.handle_key_events(key_event);
                }
                Section::Summary if key_event.code == KeyCode::Enter => {
                    let vm_build_data = self.build();
                    sender.send(Event::VMCreated(vm_build_data))?;
                }
                Section::RemoteAccess => {
                    self.remote_access.handle_key_events(key_event);
                }
                Section::Summary => {}
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
            Section::PortForwarding => {
                if is_focused {
                    Span::styled(
                        " Port Forwaring   ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from(" Port Forwaring   ").fg(Color::DarkGray)
                }
            }
            Section::RemoteAccess => {
                if is_focused {
                    Span::styled(
                        " Remote Access 󰢹 ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from(" Remote Access 󰢹 ").fg(Color::DarkGray)
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
                        self.title_span(Section::PortForwarding),
                        self.title_span(Section::RemoteAccess),
                        self.title_span(Section::Summary),
                    ])
                })
                .title_alignment(Alignment::Center)
                .padding(Padding::top(1)),
            block,
        );
    }

    pub fn render(&mut self, frame: &mut Frame, cancel_popup: bool) {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(40),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(frame.area())[1];

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(130),
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
                    if cancel_popup
                        | self.storage.new_disk_popup()
                        | self.port_fowrwaring.new_mapping_popup()
                    {
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

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(80),
                Constraint::Fill(1),
            ])
            .flex(ratatui::layout::Flex::Center)
            .split(area)[1];

        match &self.focused_section {
            Section::Overview => {
                self.overview.render(frame, area, cancel_popup);
            }
            Section::Hardware => {
                self.hardware
                    .render(frame, area, self.overview.os(), cancel_popup);
            }

            Section::Storage => {
                self.storage.render(frame, area);
            }

            Section::Network => {
                self.network.render(frame, area);
            }

            Section::PortForwarding => {
                self.port_fowrwaring.render(frame, area);
            }

            Section::RemoteAccess => {
                self.remote_access.render(frame, area, cancel_popup);
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
                items.extend(self.port_fowrwaring.summary());
                items.extend(self.remote_access.summary());

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
    }

    pub fn help(&self) -> Vec<Line<'static>> {
        match self.focused_section {
            Section::Storage => {
                if self.storage.new_disk_popup() {
                    vec![Line::from(vec![
                        Span::from("k,↑").bold(),
                        Span::from(" Up"),
                        Span::from(" | "),
                        Span::from("j,↓").bold(),
                        Span::from(" Down"),
                        Span::from(" | "),
                        Span::from("h,←").bold(),
                        Span::from(" Left"),
                        Span::from(" | "),
                        Span::from("l,→").bold(),
                        Span::from(" Right"),
                        Span::from(" | "),
                        Span::from("Esc").bold(),
                        Span::from(" Cancel"),
                        Span::from(" | "),
                        Span::from("Enter").bold(),
                        Span::from(" Confirm"),
                    ])]
                } else {
                    vec![Line::from(vec![
                        Span::from("k,↑").bold(),
                        Span::from(" Up"),
                        Span::from(" | "),
                        Span::from("j,↓").bold(),
                        Span::from(" Down"),
                        Span::from(" | "),
                        Span::from("n").bold(),
                        Span::from(" Add"),
                        Span::from(" | "),
                        Span::from("d").bold(),
                        Span::from(" Delete"),
                        Span::from(" | "),
                        Span::from("Esc").bold(),
                        Span::from(" Cancel"),
                        Span::from(" | "),
                        Span::from("⇄").bold(),
                        Span::from(" Nav"),
                    ])]
                }
            }
            Section::PortForwarding => {
                if self.port_fowrwaring.new_mapping_popup() {
                    vec![Line::from(vec![
                        Span::from("k,↑").bold(),
                        Span::from(" Up"),
                        Span::from(" | "),
                        Span::from("j,↓").bold(),
                        Span::from(" Down"),
                        Span::from(" | "),
                        Span::from("h,←").bold(),
                        Span::from(" Left"),
                        Span::from(" | "),
                        Span::from("l,→").bold(),
                        Span::from(" Right"),
                        Span::from(" | "),
                        Span::from("Esc").bold(),
                        Span::from(" Cancel"),
                        Span::from(" | "),
                        Span::from("Enter").bold(),
                        Span::from(" Confirm"),
                    ])]
                } else {
                    vec![Line::from(vec![
                        Span::from("k,↑").bold(),
                        Span::from(" Up"),
                        Span::from(" | "),
                        Span::from("j,↓").bold(),
                        Span::from(" Down"),
                        Span::from(" | "),
                        Span::from("n").bold(),
                        Span::from(" Add"),
                        Span::from(" | "),
                        Span::from("d").bold(),
                        Span::from(" Delete"),
                        Span::from(" | "),
                        Span::from("Esc").bold(),
                        Span::from(" Cancel"),
                        Span::from(" | "),
                        Span::from("⇄").bold(),
                        Span::from(" Nav"),
                    ])]
                }
            }
            Section::Summary => {
                vec![Line::from(vec![
                    Span::from("Esc").bold(),
                    Span::from(" Cancel"),
                    Span::from(" | "),
                    Span::from("Enter").bold(),
                    Span::from(" Create"),
                    Span::from(" | "),
                    Span::from("⇄").bold(),
                    Span::from(" Nav"),
                ])]
            }
            Section::Network => {
                vec![Line::from(vec![
                    Span::from("Esc").bold(),
                    Span::from(" Cancel"),
                    Span::from(" | "),
                    Span::from("⇄").bold(),
                    Span::from(" Nav"),
                ])]
            }
            _ => {
                vec![Line::from(vec![
                    Span::from("↑").bold(),
                    Span::from(" Up"),
                    Span::from(" | "),
                    Span::from("↓").bold(),
                    Span::from(" Down"),
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
                    Span::from("⇄").bold(),
                    Span::from(" Nav"),
                ])]
            }
        }
    }
}
