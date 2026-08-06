use std::{cmp::min, fs::File, io::Write, sync::mpsc::Sender};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qapi::qmp::RunState;

use crate::{
    app::{App, FocusedSection},
    confirmation::delete::DeleteConfirmation,
    event::Event::{self, NewVM},
    notification::{Notification, NotificationLevel},
    vmedit::EditVM,
};

pub fn handle_key_events(key_event: KeyEvent, app: &mut App, sender: Sender<Event>) -> Result<()> {
    match app.focused_section {
        FocusedSection::NewVM => {
            if let Some(vm_builder) = &mut app.new_vm {
                vm_builder.handle_key_events(key_event, sender)?;
            }
        }

        FocusedSection::DeleteConfirmation => {
            if let KeyCode::Esc = key_event.code {
                app.focused_section = FocusedSection::Main;
                app.delete_confirmation = None;
                return Ok(());
            }
            if let Some(delete_confirmation) = &mut app.delete_confirmation {
                delete_confirmation.handle_key_events(key_event, sender)?;
            }
        }

        FocusedSection::EditVM => {
            if let Some(edit_vm) = &mut app.edit_vm {
                if edit_vm.new_disk.is_none() && key_event.code == KeyCode::Esc {
                    app.focused_section = FocusedSection::Main;
                    app.edit_vm = None;
                } else {
                    edit_vm.handle_key_events(key_event, sender)?;
                }
            }
        }

        FocusedSection::Main => match key_event.code {
            KeyCode::Char('q') => {
                for vm in &app.vms {
                    vm.save_events_to_file()?;
                }
                app.quit();
            }

            KeyCode::Char('c') | KeyCode::Char('C')
                if key_event.modifiers == KeyModifiers::CONTROL =>
            {
                for vm in &app.vms {
                    vm.save_events_to_file()?;
                }
                app.quit();
            }

            KeyCode::Char('n') => {
                sender.send(NewVM)?;
            }

            KeyCode::Char('j') => {
                let index = match app.vm_list_state.selected() {
                    Some(i) => min(i + 1, app.vms.len() - 1),
                    None => 0,
                };
                app.vm_list_state.select(Some(index));
            }

            KeyCode::Char('k') => {
                let index = match app.vm_list_state.selected() {
                    Some(i) => i.saturating_sub(1),
                    None => 0,
                };
                app.vm_list_state.select(Some(index));
            }

            KeyCode::Char('d') => {
                if let Some(index) = app.vm_list_state.selected()
                    && let Some(vm) = app.vms.get(index)
                    && vm.state == RunState::shutdown
                {
                    app.focused_section = FocusedSection::DeleteConfirmation;
                    app.delete_confirmation = Some(DeleteConfirmation::default());
                }
            }
            KeyCode::Char('e') => {
                if let Some(index) = app.vm_list_state.selected()
                    && let Some(vm) = app.vms.get(index)
                {
                    if vm.state == RunState::shutdown {
                        app.focused_section = FocusedSection::EditVM;
                        app.edit_vm = Some(EditVM::new(vm));
                    } else {
                        let notif = Notification::new(
                            "VM should be shutdown before edit",
                            NotificationLevel::Info,
                        );

                        let _ = sender.send(Event::Notification(notif));
                    }
                }
            }

            KeyCode::Char('v') => {
                if let Some(index) = app.vm_list_state.selected()
                    && let Some(vm) = app.vms.get(index)
                {
                    let filename = "debug.txt";
                    let mut file = File::options().append(true).create(true).open(filename)?;
                    for drive in &vm.drives {
                        let arg = drive.to_qemu_arg();
                        writeln!(&mut file, "{arg}")?;
                    }
                }
            }

            _ => {
                if let Some(index) = app.vm_list_state.selected()
                    && let Some(vm) = app.vms.get_mut(index)
                {
                    vm.handle_key_events(key_event, sender)?;
                }
            }
        },
    }
    Ok(())
}
