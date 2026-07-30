use std::{cmp::min, sync::mpsc::Sender};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    app::{App, FocusedSection},
    confirmation::delete::DeleteConfirmation,
    event::Event::{self, NewVM},
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
                if app.vm_list_state.selected().is_some() {
                    app.focused_section = FocusedSection::DeleteConfirmation;
                    app.delete_confirmation = Some(DeleteConfirmation::default());
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
