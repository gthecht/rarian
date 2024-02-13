extern crate notify;
use notify::event::*;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};

use crate::gatherer::file_watcher::watch_dir_thread;

#[derive(Debug)]
enum FileEventKind {
    Access,
    Create,
    Modify,
    Remove,
}

fn create_channel() -> (
    Sender<Result<notify::Event, notify::Error>>,
    Receiver<Result<notify::Event, notify::Error>>,
) {
    return channel();
}

fn log_file(
    rx_event: Result<notify::Event, notify::Error>,
) -> Result<(Vec<PathBuf>, FileEventKind), notify::Error> {
    match rx_event {
        Ok(event) => {
            let event_paths = event.paths;
            match event.kind {
                EventKind::Access(_) => return Ok((event_paths, FileEventKind::Access)),
                EventKind::Create(_) => return Ok((event_paths, FileEventKind::Create)),
                EventKind::Modify(_) => return Ok((event_paths, FileEventKind::Modify)),
                EventKind::Remove(_) => return Ok((event_paths, FileEventKind::Remove)),
                _ => return Err(notify::Error::generic("event kind not defined")),
            }
        }
        Err(e) => {
            println!("notify error: {:?}!", e);
            return Err(e);
        }
    }
}

pub fn file_gatherer(file_path: String) -> (Sender<bool>, JoinHandle<()>) {
    let (tx, rx) = create_channel();
    let (notify_ctrl_tx, notify_ctrl_rx) = channel();
    let path: PathBuf = PathBuf::from(file_path);
    let file_watcher_thread = watch_dir_thread(path.as_path(), tx, notify_ctrl_rx);
    spawn(move || {
        loop {
            match rx.recv() {
                Ok(rx_event) => {
                    let log = log_file(rx_event);
                    match log {
                        Ok(tup) => println!("{:?}", tup),
                        Err(_e) => break,
                    }
                }
                Err(e) => {
                    println!("rx error: {:?}!", e);
                    break;
                }
            }
        }
    });
    return (notify_ctrl_tx, file_watcher_thread);
}

pub fn cleanup_file_gatherer(thread_ctrl: Sender<bool>, watcher_thread: JoinHandle<()>) {
    thread_ctrl.send(true).expect("send failed");
    watcher_thread.join().unwrap();
}
