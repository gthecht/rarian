use serde::Serialize;
use serde_json;
extern crate notify;
use anyhow::{Context, Result};
use notify::event::*;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};
use std::time::SystemTime;

use crate::gatherer::file_watcher::watch_dir_thread;
use crate::gatherer::logger::{FileLogger, Log, LogEvent};

#[derive(Debug, Serialize)]
enum FileEventKind {
    Access,
    Create,
    Modify,
    Remove,
}

#[derive(Debug, Serialize)]
struct FileEventLog {
    event_paths: Vec<PathBuf>,
    event_kind: FileEventKind,
    timestamp: SystemTime,
}

impl FileEventLog {
    fn new(tupl: (Vec<PathBuf>, FileEventKind)) -> Self {
        let (event_paths, event_kind) = tupl;
        let timestamp = SystemTime::now();
        FileEventLog { event_paths, event_kind, timestamp }
    }
}

impl LogEvent<FileLogger> for FileEventLog {
    fn log_event(&self, file_logger: &mut FileLogger) -> Result<()> {
        let json_string = serde_json::to_string(self).context("json is parsable to string")?;
        file_logger.log(json_string)
    }
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

fn create_notify_channel() -> (
    Sender<Result<notify::Event, notify::Error>>,
    Receiver<Result<notify::Event, notify::Error>>,
) {
    return channel();
}

pub fn file_gatherer(file_paths: Vec<String>, log_path: &str) -> impl FnOnce() {
    let log_path: PathBuf = PathBuf::from(log_path).join("files.json");
    let mut file_logger = FileLogger::new(log_path);

    let (notify_tx, notify_rx) = create_notify_channel();

    let gatherer_cleanup: Vec<(Sender<bool>, JoinHandle<()>)> = file_paths
        .into_iter()
        .map(|file_path| {
            let (notify_ctrl_tx, notify_ctrl_rx) = channel();
            let path: PathBuf = PathBuf::from(file_path);
            let file_watcher_thread =
                watch_dir_thread(path.as_path(), notify_tx.clone(), notify_ctrl_rx);
            return (notify_ctrl_tx, file_watcher_thread);
        })
        .collect();

    spawn(move || loop {
        match notify_rx.recv() {
            Ok(rx_event) => {
                let log = log_file(rx_event);
                match log {
                    Ok(tup) => {
                        let file_event_log = FileEventLog::new(tup);
                        file_event_log.log_event(&mut file_logger).expect("log event failed");
                    },
                    Err(_e) => break,
                }
            }
            Err(e) => {
                println!("rx error: {:?}!", e);
                break;
            }
        }
    });
    return move || {
        for (thread_ctrl, watcher_thread) in gatherer_cleanup.into_iter() {
            thread_ctrl.send(true).expect("send failed");
            watcher_thread.join().unwrap();
        }
    };
}
