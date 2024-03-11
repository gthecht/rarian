use serde::Serialize;
use serde_json;
extern crate notify;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};
use std::time::SystemTime;

use crate::gatherer::file_watcher::watch_dir_thread;
use crate::gatherer::logger::{FileLogger, Log, LogEvent};

fn create_notify_channel() -> (
    Sender<Result<notify::Event, notify::Error>>,
    Receiver<Result<notify::Event, notify::Error>>,
) {
    return channel();
}

#[derive(Debug, Serialize)]
struct FileEventLog {
    event: notify::Event,
    timestamp: SystemTime,
}

impl FileEventLog {
    fn new(event: notify::Event) -> Self {
        let timestamp = SystemTime::now();
        FileEventLog { event, timestamp }
    }
}

impl LogEvent<FileLogger> for FileEventLog {
    fn log_event(&self, file_logger: &mut FileLogger) -> Result<()> {
        let json_string = serde_json::to_string(self).context("json is parsable to string")?;
        file_logger.log(json_string)
    }
}

fn create_file_watchers(
    file_paths: Vec<String>,
    notify_tx: Sender<Result<notify::Event, notify::Error>>,
) -> impl FnOnce() {
    let file_watcher_threads: Vec<(Sender<bool>, JoinHandle<()>)> = file_paths
        .into_iter()
        .map(|file_path| {
            let (notify_ctrl_tx, notify_ctrl_rx) = channel();
            let path: PathBuf = PathBuf::from(file_path);
            let file_watcher_thread =
                watch_dir_thread(path.as_path(), notify_tx.clone(), notify_ctrl_rx);
            return (notify_ctrl_tx, file_watcher_thread);
        })
        .collect();

    let close_file_watchers = move || {
        for (thread_ctrl, watcher_thread) in file_watcher_threads.into_iter() {
            thread_ctrl.send(true).expect("send failed");
            watcher_thread.join().unwrap();
        }
    };
    return close_file_watchers;
}

pub fn file_gatherer(file_paths: Vec<String>, log_path: &str) -> impl FnOnce() {
    let log_path: PathBuf = PathBuf::from(log_path).join("files.json");
    let mut file_logger = FileLogger::new(log_path);

    let (notify_tx, notify_rx) = create_notify_channel();
    let close_file_watchers = create_file_watchers(file_paths, notify_tx);

    spawn(move || loop {
        match notify_rx.recv() {
            Ok(Ok(file_event)) => {
                let log = FileEventLog::new(file_event);
                log.log_event(&mut file_logger).expect("log event failed");
            }
            Ok(Err(e)) => {
                println!("notify error: {:?}!", e);
                break;
            }
            Err(e) => {
                println!("rx error: {:?}!", e);
                break;
            }
        }
    });

    return close_file_watchers;
}
