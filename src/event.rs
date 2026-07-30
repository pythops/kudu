use crate::{notification::Notification, vm::VM};
use anyhow::Result;
use std::{
    sync::mpsc::{Receiver, Sender, channel},
    thread,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent};

#[derive(Clone)]
pub enum DownloadEvent {
    Progress(u8),
    Error(String),
}

#[derive(Clone)]
pub enum Event {
    Tick,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Notification(Notification),
    NewVM,
    CancelVM(bool),
    VMCreated(VM),
    Download((uuid::Uuid, DownloadEvent)),
    VMStarted(uuid::Uuid),
    QuitVM(uuid::Uuid),
    DeleteVm(bool),
    QemuEvent((uuid::Uuid, qapi::qmp::Event)),
    QemuExit(uuid::Uuid),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct EventHandler {
    pub sender: Sender<Event>,
    pub receiver: Receiver<Event>,
    pub handler: thread::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: u64) -> Self {
        let tick_rate = Duration::from_millis(tick_rate);
        let (sender, receiver) = channel();
        let handler = {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut last_tick = Instant::now();
                loop {
                    let timeout = tick_rate
                        .checked_sub(last_tick.elapsed())
                        .unwrap_or(tick_rate);

                    if event::poll(timeout).expect("failed to poll new events") {
                        match event::read().expect("unable to read event") {
                            CrosstermEvent::Key(e) => {
                                if e.kind == KeyEventKind::Press {
                                    sender.send(Event::Key(e))
                                } else {
                                    Ok(())
                                }
                            }
                            CrosstermEvent::Mouse(e) => sender.send(Event::Mouse(e)),
                            CrosstermEvent::Resize(w, h) => sender.send(Event::Resize(w, h)),
                            CrosstermEvent::FocusGained => Ok(()),
                            CrosstermEvent::FocusLost => Ok(()),
                            CrosstermEvent::Paste(_) => unimplemented!(),
                        }
                        .expect("failed to send terminal event")
                    }

                    if last_tick.elapsed() >= tick_rate {
                        sender.send(Event::Tick).expect("failed to send tick event");
                        last_tick = Instant::now();
                    }
                }
            })
        };
        Self {
            sender,
            receiver,
            handler,
        }
    }

    pub fn next(&self) -> Result<Event> {
        Ok(self.receiver.recv()?)
    }
}
