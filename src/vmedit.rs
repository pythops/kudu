use anyhow::Result;
use std::{mem::discriminant, path::PathBuf, sync::mpsc::Sender};

use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Padding, Row, Table, TableState},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    Arch,
    event::Event,
    network::{MappingBuilder, PortMapping},
    storage::{Disk, DiskBuilder, Media},
    vm::VM,
};

#[derive(Debug, Clone, Default)]
struct UserInputField {
    field: Input,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Section {
    Hardware(HardwareSection),
    Storage,
    PortForwarding,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum HardwareSection {
    #[default]
    Cpu,
    Memory,
}

#[derive(Debug, Clone)]
pub struct EditVM {
    _name: UserInputField,
    vcpu: UserInputField,
    memory: UserInputField,
    disk_state: TableState,
    pub new_disk: Option<DiskBuilder>,
    pub focused_section: Section,
    added_disks: Vec<Disk>,
    deleted_disks: Vec<PathBuf>,
    added_port_mappings: Vec<PortMapping>,
    deleted_port_mappings: Vec<PortMapping>,
    pub new_mapping: Option<MappingBuilder>,
    mapping_state: TableState,
    pub vm: VM,
}

#[derive(Debug, Clone)]
pub struct VMEditData {
    pub id: uuid::Uuid,
    pub deleted_disks: Vec<PathBuf>,
    pub added_disks: Vec<Disk>,
    pub new_vcpu: u16,
    pub new_memory: u32,
    pub port_mappings: Vec<PortMapping>,
}

impl EditVM {
    pub fn new(vm: &VM) -> Self {
        let disks = vm.disks();
        let disk_state = if disks.is_empty() {
            TableState::default()
        } else {
            TableState::default().with_selected(Some(0))
        };

        let mapping_state = if vm.port_mappings.is_empty() {
            TableState::default()
        } else {
            TableState::default().with_selected(Some(0))
        };

        Self {
            _name: UserInputField {
                field: Input::from(vm.name.clone()),
                error: None,
            },
            vcpu: UserInputField {
                field: Input::from(vm.vcpu.to_string()),
                error: None,
            },
            memory: UserInputField {
                field: Input::from(vm.memory.to_string()),
                error: None,
            },
            disk_state,
            new_disk: None,
            focused_section: Section::Hardware(HardwareSection::Cpu),
            added_disks: Vec::new(),
            deleted_disks: Vec::new(),
            added_port_mappings: Vec::new(),
            deleted_port_mappings: Vec::new(),
            new_mapping: None,
            mapping_state,
            vm: vm.clone(),
        }
    }

    fn validate_harware_section(&mut self) -> bool {
        let mut valid = true;

        self.vcpu.error = None;
        self.memory.error = None;

        if self.vcpu.field.value().is_empty() {
            self.vcpu.error = Some("Field required".into());
            valid = false;
        } else {
            match self.vcpu.field.value().parse::<u16>() {
                Ok(v) => {
                    if v == 0 {
                        self.vcpu.error = Some("vcpu value can not be 0".into());
                        valid = false;
                    }
                }
                Err(_) => {
                    self.vcpu.error = Some("vcpu value should be a number".into());
                    valid = false;
                }
            }
        };

        if self.memory.field.value().is_empty() {
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
        };
        valid
    }

    pub fn handle_key_events(
        &mut self,
        key_event: KeyEvent,
        arch: Arch,
        sender: Sender<Event>,
    ) -> Result<()> {
        if let Some(new_disk) = &mut self.new_disk {
            match key_event.code {
                KeyCode::Esc => {
                    self.new_disk = None;
                }
                KeyCode::Enter => {
                    if new_disk.validate() {
                        let disk = new_disk.build();
                        self.added_disks.push(disk);
                        self.new_disk = None;

                        if self.disk_state.selected().is_none() {
                            self.disk_state.select(Some(0));
                        }
                    }
                }

                _ => {
                    new_disk.handle_key_events(key_event, arch);
                }
            }

            return Ok(());
        }

        if let Some(new_mapping) = &mut self.new_mapping {
            match key_event.code {
                KeyCode::Esc => {
                    self.new_mapping = None;
                }
                KeyCode::Enter => {
                    if new_mapping.validate() {
                        let mapping = new_mapping.build();
                        self.added_port_mappings.push(mapping);
                        self.new_mapping = None;

                        if self.mapping_state.selected().is_none() {
                            self.mapping_state.select(Some(0));
                        }
                    }
                }

                _ => {
                    new_mapping.handle_key_events(key_event);
                }
            }

            return Ok(());
        }

        match key_event.code {
            KeyCode::Enter => {
                let port_mappings = {
                    let mut port_mappings = self.vm.port_mappings.clone();
                    for deleted_mapping in &self.deleted_port_mappings {
                        port_mappings.retain(|mapping| mapping != deleted_mapping);
                    }
                    port_mappings.extend(self.added_port_mappings.clone());
                    port_mappings
                };

                let _ = sender.send(Event::VMEdited(VMEditData {
                    id: self.vm.id,
                    deleted_disks: self.deleted_disks.clone(),
                    added_disks: self.added_disks.clone(),
                    new_vcpu: self.vcpu.field.value().parse::<u16>().unwrap(),
                    new_memory: self.memory.field.value().parse::<u32>().unwrap(),
                    port_mappings,
                }));
            }
            KeyCode::Tab => match self.focused_section {
                Section::Hardware(_) => {
                    if self.validate_harware_section() {
                        self.focused_section = Section::Storage;
                    }
                }
                Section::Storage => {
                    self.focused_section = Section::PortForwarding;
                }
                Section::PortForwarding => {
                    self.focused_section = Section::Hardware(HardwareSection::default())
                }
            },
            KeyCode::BackTab => match self.focused_section {
                Section::Hardware(_) => {
                    if self.validate_harware_section() {
                        self.focused_section = Section::PortForwarding;
                    }
                }
                Section::Storage => {
                    self.focused_section = Section::Hardware(HardwareSection::default())
                }
                Section::PortForwarding => {
                    self.focused_section = Section::Storage;
                }
            },
            _ => match &self.focused_section {
                Section::Hardware(hardware_section) => match hardware_section {
                    HardwareSection::Cpu => match key_event.code {
                        KeyCode::Up if self.validate_harware_section() => {
                            self.focused_section = Section::Hardware(HardwareSection::Memory);
                        }
                        KeyCode::Down if self.validate_harware_section() => {
                            self.focused_section = Section::Hardware(HardwareSection::Memory);
                        }
                        _ => {
                            self.vcpu
                                .field
                                .handle_event(&crossterm::event::Event::Key(key_event));
                        }
                    },
                    HardwareSection::Memory => match key_event.code {
                        KeyCode::Up if self.validate_harware_section() => {
                            self.focused_section = Section::Hardware(HardwareSection::Cpu);
                        }
                        KeyCode::Down if self.validate_harware_section() => {
                            self.focused_section = Section::Hardware(HardwareSection::Cpu);
                        }
                        _ => {
                            self.memory
                                .field
                                .handle_event(&crossterm::event::Event::Key(key_event));
                        }
                    },
                },
                Section::Storage => match key_event.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        if let Some(index) = self.disk_state.selected() {
                            self.disk_state.select(Some(
                                index
                                    .saturating_add(1)
                                    .min(self.vm.disks().len() + self.added_disks.len() - 1),
                            ));
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if let Some(index) = self.disk_state.selected() {
                            self.disk_state.select(Some(index.saturating_sub(1)));
                        }
                    }
                    KeyCode::Char('d') => {
                        if let Some(index) = self.disk_state.selected() {
                            let vm_disks = self.vm.disks();

                            if index < vm_disks.len() {
                                let disk = &vm_disks[index];
                                self.deleted_disks.push(disk.path.clone());
                            } else {
                                let index = index.saturating_sub(vm_disks.len());
                                self.added_disks.remove(index);
                            }
                        }
                    }
                    KeyCode::Char('u') => {
                        if let Some(index) = self.disk_state.selected()
                            && let Some(disk) = self.vm.disks().get(index)
                        {
                            self.deleted_disks.retain(|path| &disk.path != path);
                        }
                    }
                    KeyCode::Char('n') => {
                        self.new_disk = Some(DiskBuilder::new());
                    }
                    _ => {}
                },
                Section::PortForwarding => match key_event.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        if let Some(index) = self.mapping_state.selected() {
                            self.mapping_state.select(Some(index.saturating_add(1).min(
                                self.vm.port_mappings.len() + self.added_port_mappings.len() - 1,
                            )));
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if let Some(index) = self.mapping_state.selected() {
                            self.mapping_state.select(Some(index.saturating_sub(1)));
                        }
                    }
                    KeyCode::Char('d') => {
                        if let Some(index) = self.mapping_state.selected() {
                            if index < self.vm.port_mappings.len() {
                                let mapping = &self.vm.port_mappings[index];
                                if !self.deleted_port_mappings.contains(mapping) {
                                    self.deleted_port_mappings.push(mapping.clone());
                                }
                            } else {
                                let index = index.saturating_sub(self.vm.port_mappings.len());
                                self.added_port_mappings.remove(index);
                            }
                        }
                    }
                    KeyCode::Char('u') => {
                        if let Some(index) = self.mapping_state.selected()
                            && let Some(mapping) = self.vm.port_mappings.get(index)
                        {
                            self.deleted_port_mappings
                                .retain(|deleted_mapping| deleted_mapping != mapping);
                        }
                    }
                    KeyCode::Char('n') => {
                        self.new_mapping = Some(MappingBuilder::new());
                    }
                    _ => {}
                },
            },
        }

        Ok(())
    }

    fn title_span(&self, section: Section) -> Span<'_> {
        let is_focused = discriminant(&self.focused_section) == discriminant(&section);
        match section {
            Section::Hardware(_) => {
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
        }
    }
    fn render_header(&self, frame: &mut Frame, block: Rect) {
        frame.render_widget(
            Block::default()
                .title({
                    Line::from(vec![
                        self.title_span(Section::Hardware(HardwareSection::default())),
                        self.title_span(Section::Storage),
                        self.title_span(Section::PortForwarding),
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
                Constraint::Length(30),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(frame.area())[1];

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(90),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(area)[1];

        frame.render_widget(Clear, area);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(" Edit VM 󰏖  ")
                .title_alignment(Alignment::Center)
                .border_type(BorderType::Thick)
                .border_style(Style::default().yellow()),
            area,
        );

        let area = area.inner(Margin {
            horizontal: 3,
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
            Section::Hardware(hardware_section) => {
                let (cpu_block, memory_block) = {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(4), Constraint::Length(4)])
                        .margin(2)
                        .split(area);

                    (chunks[0], chunks[1])
                };
                let cpu = vec![
                    Line::from(vec![
                        {
                            if hardware_section == &HardwareSection::Cpu {
                                Span::from("> CPU   ").bold()
                            } else {
                                Span::from("  CPU   ")
                            }
                        },
                        Span::from(" ".repeat(6)),
                        Span::from({
                            let original_length = self.vcpu.field.to_string().len();
                            let target_length = 60_usize;

                            self.vcpu
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
                        Span::from(" ".repeat(14)),
                        Span::from(self.vcpu.clone().error.unwrap_or("".to_string())).red(),
                    ]),
                ];

                let memory = vec![
                    Line::from(vec![
                        {
                            if hardware_section == &HardwareSection::Memory {
                                Span::from("> Memory").bold()
                            } else {
                                Span::from("  Memory")
                            }
                        },
                        Span::from(" ".repeat(6)),
                        Span::from({
                            let original_length = self.memory.field.to_string().len();
                            let target_length = 60_usize;

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
                        Span::from(" MB"),
                    ]),
                    Line::from(vec![
                        Span::from(" ".repeat(14)),
                        Span::from(self.memory.clone().error.unwrap_or("".to_string())).red(),
                    ]),
                ];
                frame.render_widget(Text::from(cpu), cpu_block);
                frame.render_widget(Text::from(memory), memory_block);
            }
            Section::Storage => {
                let area = area.inner(Margin {
                    horizontal: 2,
                    vertical: 2,
                });

                // disks
                let widths = [
                    Constraint::Length(5),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(15),
                ];

                let vm_disks = self.vm.disks();
                let vm_disks = vm_disks.iter().map(|disk| {
                    let to_delete = self.deleted_disks.contains(&disk.path);
                    Row::new(vec![
                        {
                            if to_delete {
                                "Del".to_string()
                            } else {
                                String::new()
                            }
                        },
                        {
                            match disk.media {
                                Media::Disk => "Disk    ".to_string(),
                                Media::CdRom => "Cdrom   ".to_string(),
                            }
                        },
                        disk.format.to_string(),
                        {
                            if let Some(size) = disk.size {
                                format!("{} GiB", size)
                            } else {
                                "-".to_string()
                            }
                        },
                        disk.interface.to_string(),
                        {
                            match disk.media {
                                Media::Disk => "-".to_string(),
                                Media::CdRom => disk
                                    .path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                            }
                        },
                    ])
                    .style(if to_delete {
                        Style::new().red()
                    } else {
                        Style::default()
                    })
                });

                let new_disks = self.added_disks.iter().map(|disk| {
                    Row::new(vec![
                        "New".to_string(),
                        "Disk    ".to_string(),
                        disk.format.to_string(),
                        format!("{} GiB", disk.size),
                        disk.interface.to_string(),
                        "-".to_string(),
                    ])
                    .green()
                });

                let mut drives: Vec<Row> = Vec::new();
                drives.extend(vm_disks);
                drives.extend(new_disks);

                let disks = Table::new(drives, widths)
                    .header(
                        Row::new(vec!["", "Type", "Format", "Size", "Interface", "File Name"])
                            .style(Style::new().bold())
                            .bottom_margin(1),
                    )
                    .flex(ratatui::layout::Flex::SpaceBetween)
                    .row_highlight_style(Style::new().on_dark_gray())
                    .column_spacing(1);

                frame.render_stateful_widget(
                    disks,
                    area.inner(Margin {
                        horizontal: 0,
                        vertical: 2,
                    }),
                    &mut self.disk_state,
                );

                if let Some(new_disk) = &self.new_disk {
                    new_disk.render(frame);
                }
            }
            Section::PortForwarding => {
                let area = area.inner(Margin {
                    horizontal: 2,
                    vertical: 2,
                });

                let widths = [
                    Constraint::Length(5),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                ];

                let vm_port_mappings = self.vm.port_mappings.clone();

                let vm_port_mappings = vm_port_mappings.iter().map(|mapping| {
                    let to_delete = self.deleted_port_mappings.contains(mapping);
                    Row::new(vec![
                        {
                            if to_delete {
                                "Del".to_string()
                            } else {
                                String::new()
                            }
                        },
                        mapping.protocol.to_string(),
                        mapping.guest_port.to_string(),
                        mapping.host_port.to_string(),
                    ])
                    .style(if to_delete {
                        Style::new().red()
                    } else {
                        Style::default()
                    })
                });

                let new_port_mappings = self.added_port_mappings.iter().map(|mapping| {
                    Row::new(vec![
                        "New".to_string(),
                        mapping.protocol.to_string(),
                        mapping.guest_port.to_string(),
                        mapping.host_port.to_string(),
                    ])
                    .green()
                });

                let mut mappings: Vec<Row> = Vec::new();
                mappings.extend(vm_port_mappings);
                mappings.extend(new_port_mappings);

                let mappings = Table::new(mappings, widths)
                    .header(
                        Row::new(vec!["", "Protocol", "Guest Port", "Host Port"])
                            .style(Style::new().bold())
                            .bottom_margin(1),
                    )
                    .flex(ratatui::layout::Flex::SpaceBetween)
                    .row_highlight_style(Style::new().on_dark_gray())
                    .column_spacing(1);

                frame.render_stateful_widget(mappings, area, &mut self.mapping_state);

                if let Some(new_mapping) = &self.new_mapping {
                    new_mapping.render(frame);
                }
            }
        }
    }
}
