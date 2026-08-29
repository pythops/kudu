use std::{io, thread, time::Duration};

use anyhow::Result;
use chrono::{DateTime, Utc};
use kudu::{
    USER_UID,
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
    vm::VM,
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

    if std::env::consts::OS != "linux" {
        println!("kudu only runs on Linux");
        return Ok(());
    }

    unsafe { USER_UID = libc::geteuid() };

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
                        vmbuilder.cancel_confirmation = None;
                    }
                }
            }

            Event::VMCreated(data) => {
                match VM::create(data) {
                    Ok(vm) => {
                        app.vms.push(vm);
                        if app.vm_list_state.selected().is_none() {
                            app.vm_list_state.select(Some(0));
                        }
                    }
                    Err(error) => {
                        let notif =
                            kudu::notification::Notification::new(error, NotificationLevel::Error);
                        let _ = tui.events.sender.send(Notification(notif));
                    }
                }

                app.focused_section = FocusedSection::Main;
                app.new_vm = None;
            }

            Event::VMEdited(data) => {
                if let Some(vm) = app.vms.iter_mut().find(|vm| vm.id == data.id) {
                    if let Err(error) = vm.edit(data) {
                        let notif =
                            kudu::notification::Notification::new(error, NotificationLevel::Error);

                        let _ = tui.events.sender.send(Notification(notif));
                    }
                    app.edit_vm = None;
                    app.focused_section = FocusedSection::Main;
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

                            if progress == 100 {
                                thread::sleep(Duration::from_secs(1));
                                let _ = vm.start(tui.events.sender.clone());
                            }
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

            Event::DeleteVm(choice) => {
                if let Some(index) = app.vm_list_state.selected()
                    && let Some(vm) = app.vms.get_mut(index)
                    && choice
                {
                    if let Err(error) = vm.delete() {
                        let notif =
                            kudu::notification::Notification::new(error, NotificationLevel::Error);

                        let _ = tui.events.sender.send(Notification(notif));
                    }
                    let _ = app.vms.remove(index);
                }
                app.delete_confirmation = None;
                app.focused_section = FocusedSection::Main;
            }

            Event::QemuEvent((id, event)) => {
                if let Some(vm) = app.vms.iter_mut().find(|m| m.id == id) {
                    let message = match &event {
                        qapi::qmp::Event::STOP { .. } => {
                            vm.state = RunState::paused;
                            "PAUSE".to_string()
                        }
                        qapi::qmp::Event::RESUME { .. } => {
                            vm.state = RunState::running;
                            "RESUME".to_string()
                        }
                        qapi::qmp::Event::SUSPEND { .. } => {
                            vm.state = RunState::suspended;
                            "SUSPEND".to_string()
                        }
                        qapi::qmp::Event::SHUTDOWN { .. } => {
                            if let Err(error) = vm.shutdown() {
                                let notif = kudu::notification::Notification::new(
                                    error,
                                    NotificationLevel::Error,
                                );

                                let _ = tui.events.sender.send(Notification(notif));
                            }
                            "SHUTDOWN".to_string()
                        }
                        qapi::qmp::Event::VNC_CONNECTED { data, .. } => {
                            let client_ip = &data.client.host;
                            format!("New VNC connection established from {}", client_ip)
                        }
                        qapi::qmp::Event::VNC_DISCONNECTED { data, .. } => {
                            let client_ip = &data.client.base.host;
                            format!("VNC session terminated from {}", client_ip)
                        }
                        _ => {
                            let event = serde_json::to_value(&event).unwrap();
                            let event_name = event["event"].to_string().replace("_", " ");
                            event_name.trim_matches('\"').to_string()
                        }
                    };

                    let date = {
                        let event = serde_json::to_value(&event).unwrap();
                        let seconds = event["timestamp"]["seconds"].as_i64().unwrap_or(0);
                        let microseconds = event["timestamp"]["microseconds"].as_u64().unwrap_or(0);
                        let nanoseconds = (microseconds * 1000) as u32;

                        let dt = DateTime::from_timestamp(seconds, nanoseconds).unwrap();
                        dt.format("%Y-%m-%d %H:%M:%S").to_string()
                    };

                    vm.events.push(format!("{} - {}", date, message));
                }
            }

            Event::QemuExit(id) => {
                if let Some(vm) = app.vms.iter_mut().find(|m| m.id == id)
                    && vm.state != RunState::shutdown
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
