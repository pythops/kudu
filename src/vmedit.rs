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
    event::Event,
    storage::{Disk, DiskBuilder},
    vm::VM,
};

#[derive(Debug, Clone, Default)]
struct UserInputField {
    field: Input,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum Section {
    Hardware(HardwareSection),
    Storage(StorageSection),
}

#[derive(Debug, Clone, Default, PartialEq)]
enum StorageSection {
    #[default]
    Disk,
    Cdrom,
}

#[derive(Debug, Default, Clone, PartialEq)]
enum HardwareSection {
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
    new_disk: Option<DiskBuilder>,
    focused_section: Section,
    added_disks: Vec<Disk>,
    deleted_disks: Vec<PathBuf>,
    deleted_cdroms: Vec<PathBuf>,
    cdrom_state: TableState,
    vm: VM,
}

#[derive(Debug, Clone)]
pub struct VMEditData {
    pub id: uuid::Uuid,
    pub deleted_disks: Vec<PathBuf>,
    pub added_disks: Vec<Disk>,
    pub new_vcpu: u16,
    pub new_memory: u32,
    pub delete_cdroms: Vec<PathBuf>,
}

impl EditVM {
    pub fn new(vm: &VM) -> Self {
        let disks = vm.disks();
        let disk_state = if disks.is_empty() {
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
            deleted_cdroms: Vec::new(),
            cdrom_state: TableState::default(),
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

    pub fn handle_key_events(&mut self, key_event: KeyEvent, sender: Sender<Event>) -> Result<()> {
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
                    new_disk.handle_key_events(key_event);
                }
            }

            return Ok(());
        }

        match key_event.code {
            KeyCode::Enter => {
                let _ = sender.send(Event::VMEdited(VMEditData {
                    id: self.vm.id,
                    deleted_disks: self.deleted_disks.clone(),
                    added_disks: self.added_disks.clone(),
                    new_vcpu: self.vcpu.field.value().parse::<u16>().unwrap(),
                    new_memory: self.memory.field.value().parse::<u32>().unwrap(),
                    delete_cdroms: self.deleted_cdroms.clone(),
                }));
            }
            KeyCode::Tab | KeyCode::BackTab => match self.focused_section {
                Section::Hardware(_) => {
                    if self.validate_harware_section() {
                        self.focused_section = Section::Storage(StorageSection::default());
                    }
                }
                Section::Storage(_) => {
                    self.focused_section = Section::Hardware(HardwareSection::default())
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
                Section::Storage(storage_section) => match storage_section {
                    StorageSection::Disk => match key_event.code {
                        KeyCode::Down | KeyCode::Up => {
                            self.focused_section = Section::Storage(StorageSection::Cdrom);
                            if self.vm.cdroms().is_empty() {
                                self.cdrom_state = TableState::default()
                            } else {
                                self.cdrom_state = TableState::default().with_selected(Some(0));
                            }
                            self.disk_state = TableState::default();
                        }

                        KeyCode::Char('j') => {
                            if let Some(index) = self.disk_state.selected() {
                                self.disk_state.select(Some(
                                    index
                                        .saturating_add(1)
                                        .min(self.vm.disks().len() + self.added_disks.len() - 1),
                                ));
                            }
                        }
                        KeyCode::Char('k') => {
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
                            if let Some(index) = self.disk_state.selected() {
                                let vm_disks = self.vm.disks();

                                if index < vm_disks.len() {
                                    let disk = &vm_disks[index];
                                    self.deleted_disks.retain(|path| &disk.path != path);
                                }
                            }
                        }
                        KeyCode::Char('n') => {
                            self.new_disk = Some(DiskBuilder::new());
                        }
                        _ => {}
                    },

                    StorageSection::Cdrom => match key_event.code {
                        KeyCode::Up | KeyCode::Down => {
                            self.focused_section = Section::Storage(StorageSection::Disk);
                            if self.vm.disks().is_empty() && self.added_disks.is_empty() {
                                self.disk_state = TableState::default()
                            } else {
                                self.disk_state = TableState::default().with_selected(Some(0));
                            }
                            self.cdrom_state = TableState::default();
                        }
                        KeyCode::Char('j') => {
                            if let Some(index) = self.cdrom_state.selected() {
                                self.cdrom_state.select(Some(
                                    index.saturating_add(1).min(self.vm.cdroms().len() - 1),
                                ));
                            }
                        }
                        KeyCode::Char('k') => {
                            if let Some(index) = self.cdrom_state.selected() {
                                self.cdrom_state.select(Some(index.saturating_sub(1)));
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(index) = self.cdrom_state.selected() {
                                let cdroms = self.vm.cdroms();

                                let cdrom = &cdroms[index];
                                self.deleted_cdroms.push(cdrom.path.clone());
                            }
                        }
                        KeyCode::Char('u') => {
                            if let Some(index) = self.cdrom_state.selected() {
                                let cdroms = self.vm.cdroms();
                                let cdrom = &cdroms[index];
                                self.deleted_cdroms.retain(|path| &cdrom.path != path);
                            }
                        }
                        _ => {}
                    },
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
                        "  Hardware  ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from("  Hardware  ").fg(Color::DarkGray)
                }
            }
            Section::Storage(_) => {
                if is_focused {
                    Span::styled(
                        "    Storage    ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from("    Storage    ").fg(Color::DarkGray)
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
                        self.title_span(Section::Storage(StorageSection::default())),
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
                .title(" Edit VM ")
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
                            let target_length = 50;

                            self.vcpu
                                .field
                                .to_string()
                                .chars()
                                .chain(std::iter::repeat_n(' ', target_length - original_length))
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
                            let target_length = 47;

                            self.memory
                                .field
                                .to_string()
                                .chars()
                                .chain(std::iter::repeat_n(' ', target_length - original_length))
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
            Section::Storage(storage_section) => {
                let area = area.inner(Margin {
                    horizontal: 2,
                    vertical: 2,
                });

                let (disk_area, cdrom_area) = {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .margin(1)
                        .split(area);

                    (chunks[0], chunks[1])
                };

                frame.render_widget(
                    Block::new()
                        .borders(Borders::TOP)
                        .border_type({
                            if storage_section == &StorageSection::Disk {
                                BorderType::Thick
                            } else {
                                BorderType::Plain
                            }
                        })
                        .border_style({
                            if storage_section == &StorageSection::Disk {
                                Style::new().yellow()
                            } else {
                                Style::default()
                            }
                        })
                        .title(" Disk ")
                        .title_alignment(Alignment::Center),
                    disk_area,
                );

                frame.render_widget(
                    Block::new()
                        .borders(Borders::TOP)
                        .border_type({
                            if storage_section == &StorageSection::Cdrom {
                                BorderType::Thick
                            } else {
                                BorderType::Plain
                            }
                        })
                        .border_style({
                            if storage_section == &StorageSection::Cdrom {
                                Style::new().yellow()
                            } else {
                                Style::default()
                            }
                        })
                        .title(" Cdrom ")
                        .title_alignment(Alignment::Center),
                    cdrom_area,
                );

                // disks
                let widths = [
                    Constraint::Length(5),
                    Constraint::Length(10),
                    Constraint::Length(10),
                ];

                let vm_disks = self.vm.disks();
                let vm_disks = vm_disks.iter().map(|drive| {
                    let size = drive.size.unwrap();
                    let to_delete = self.deleted_disks.contains(&drive.path);
                    Row::new(vec![
                        {
                            if to_delete {
                                "Del".to_string()
                            } else {
                                String::new()
                            }
                        },
                        drive.format.to_string(),
                        format!("{} GiB", size),
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
                        disk.format.to_string(),
                        format!("{} GiB", disk.size),
                    ])
                    .green()
                });

                let mut disks: Vec<Row> = Vec::new();
                disks.extend(vm_disks);
                disks.extend(new_disks);

                let disks = Table::new(disks, widths)
                    .header(
                        Row::new(vec!["", "Format", "Size"])
                            .style(Style::new().bold())
                            .bottom_margin(1),
                    )
                    .flex(ratatui::layout::Flex::SpaceBetween)
                    .row_highlight_style(Style::new().on_dark_gray())
                    .column_spacing(1);

                frame.render_stateful_widget(
                    disks,
                    disk_area.inner(Margin {
                        horizontal: 0,
                        vertical: 2,
                    }),
                    &mut self.disk_state,
                );

                // Cdrom
                let widths = [Constraint::Length(5), Constraint::Length(40)];

                let cdroms = self.vm.cdroms();
                let cdroms = cdroms.iter().map(|drive| {
                    let to_delete = self.deleted_cdroms.contains(&drive.path);
                    Row::new(vec![
                        {
                            if to_delete {
                                "Del".to_string()
                            } else {
                                String::new()
                            }
                        },
                        drive
                            .path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                    ])
                    .style(if to_delete {
                        Style::new().red()
                    } else {
                        Style::default()
                    })
                });

                let cdroms = Table::new(cdroms, widths)
                    .flex(ratatui::layout::Flex::SpaceBetween)
                    .row_highlight_style(Style::new().on_dark_gray())
                    .column_spacing(1);

                frame.render_stateful_widget(
                    cdroms,
                    cdrom_area.inner(Margin {
                        horizontal: 0,
                        vertical: 2,
                    }),
                    &mut self.cdrom_state,
                );

                if let Some(new_disk) = &self.new_disk {
                    new_disk.render(frame);
                }
            }
        }
    }
}
