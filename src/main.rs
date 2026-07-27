use std::io;

use anyhow::Result;
use chrono::{DateTime, Utc};
use kudu::{
    app::{App, FocusedSection},
    event::{
        DownloadEvent,
        Event::{self, Notification},
        EventHandler,
    },
    handlers::handle_key_events,
    notification::NotificationLevel,
    qemu::Qemu,
    tui::Tui,
    vmbuilder::VMBuilder,
};
use qapi::qmp::RunState;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use clap::{Command, crate_description, crate_name, crate_version};

fn main() -> Result<()> {
    Command::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .get_matches();

    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(500);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    let mut app = match App::new(tui.events.sender.clone()) {
        Ok(app) => app,
        Err(e) => {
            tui.exit()?;
            eprintln!("{e}");
            return Ok(());
        }
    };

    while app.running {
        tui.draw(&mut app)?;
        match tui.events.next()? {
            Event::Tick => app.tick(),

            Event::Key(key_event) => {
                let _ = handle_key_events(key_event, &mut app, tui.events.sender.clone());
            }

            Event::Notification(notification) => {
                app.notifications.push(notification);
            }

            Event::NewVM => {
                app.focused_section = FocusedSection::NewVM;
                app.new_vm = Some(VMBuilder::new());
            }

            Event::CancelVM(choice) => {
                if choice {
                    app.focused_section = FocusedSection::Main;
                    app.new_vm = None;
                } else {
                    if let Some(vmbuilder) = &mut app.new_vm {
                        vmbuilder.confirmation = None;
                    }
                }
            }

            Event::VMCreated(mut vm) => {
                if let Err(error) = vm.create() {
                    let notif =
                        kudu::notification::Notification::new(error, NotificationLevel::Error);
                    let _ = tui.events.sender.send(Notification(notif));
                }
                app.focused_section = FocusedSection::Main;
                app.new_vm = None;
                app.vms.push(vm);
                if app.vm_list_state.selected().is_none() {
                    app.vm_list_state.select(Some(0));
                }
            }

            Event::Download((id, event)) => {
                if let Some(vm) = app.vms.iter_mut().find(|m| m.id == id) {
                    let date = Utc::now().format("%Y-%m-%d %H:%M:%S");
                    match event {
                        DownloadEvent::Progress(progress) => {
                            vm.events.pop();
                            vm.events
                                .push(format!("{} - Downloading cloud image {}%", date, progress));
                        }
                        DownloadEvent::Error(error) => {
                            vm.events.push(format!("{} - Error - {}%", date, error));
                        }
                    }
                }
            }

            Event::VMStarted(id) => {
                if let Some(vm) = app.vms.iter_mut().find(|m| m.id == id)
                    && let Ok(state) = Qemu::status(id)
                {
                    let now = Utc::now().format("%Y-%m-%d %H:%M:%S");
                    let event = format!("{} - RUNNING", now);
                    vm.events.push(event);

                    vm.state = state;
                    if let Ok(vnc_info) = Qemu::vnc_infos(id) {
                        vm.vnc = Some(vnc_info);
                    }
                }
            }

            Event::DeleteVm(id) => {
                if let Some(index) = app.vms.iter_mut().position(|v| v.id == id) {
                    if let Some(vm) = app.vms.get_mut(index)
                        && let Err(error) = vm.delete()
                    {
                        let notif =
                            kudu::notification::Notification::new(error, NotificationLevel::Error);

                        let _ = tui.events.sender.send(Notification(notif));
                    }
                    let _ = app.vms.remove(index);
                }
            }

            Event::QemuEvent((id, event)) => {
                if let Some(vm) = app.vms.iter_mut().find(|m| m.id == id) {
                    match &event {
                        qapi::qmp::Event::STOP { .. } => {
                            vm.state = RunState::paused;
                        }
                        qapi::qmp::Event::RESUME { .. } => {
                            vm.state = RunState::running;
                        }
                        qapi::qmp::Event::SUSPEND { .. } => {
                            vm.state = RunState::suspended;
                        }
                        qapi::qmp::Event::SHUTDOWN { .. } => {
                            if let Err(error) = vm.shutdown() {
                                let notif = kudu::notification::Notification::new(
                                    error,
                                    NotificationLevel::Error,
                                );

                                let _ = tui.events.sender.send(Notification(notif));
                            }
                        }
                        _ => {}
                    };

                    let event = serde_json::to_value(event).unwrap();
                    let event_name = event["event"].to_string().replace("_", " ");
                    let event_name = event_name.trim_matches('\"');
                    let event_name = if event_name == "STOP" {
                        "PAUSE"
                    } else {
                        event_name
                    };

                    let date = {
                        let seconds = event["timestamp"]["seconds"].as_i64().unwrap_or(0);
                        let microseconds = event["timestamp"]["microseconds"].as_u64().unwrap_or(0);
                        let nanoseconds = (microseconds * 1000) as u32;

                        let dt = DateTime::from_timestamp(seconds, nanoseconds).unwrap();
                        dt.format("%Y-%m-%d %H:%M:%S").to_string()
                    };

                    vm.events.push(format!("{} - {}", date, event_name));
                }
            }

            Event::QemuExit(id) => {
                if let Some(vm) = app.vms.iter_mut().find(|m| m.id == id)
                    && let Err(error) = vm.shutdown()
                {
                    let notif =
                        kudu::notification::Notification::new(error, NotificationLevel::Error);

                    let _ = tui.events.sender.send(Notification(notif));
                }
            }
            _ => {}
        }
    }

    tui.exit()?;

    Ok(())
}
