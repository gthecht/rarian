use serde_json;
extern crate notify;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};
use std::time::SystemTime;

use crate::gatherer::file_watcher::watch_dir_thread;
use crate::gatherer::logger::{FileLogger, Log, LogEvent};

impl LogEvent<FileLogger> for notify::Event {
    fn log(&self, file_logger: &mut FileLogger) -> Result<()> {
        let timestamp = SystemTime::now();
        let log_json = serde_json::json!({
            "event": self,
            "timestamp": timestamp
        });
        file_logger.log(log_json.to_string())
    }
}

fn create_notify_channel() -> (
    Sender<Result<notify::Event, notify::Error>>,
    Receiver<Result<notify::Event, notify::Error>>,
) {
    return channel();
}

fn create_file_watchers(
    file_paths: Vec<String>,
    notify_tx: Sender<Result<notify::Event, notify::Error>>,
) -> Vec<(Sender<bool>, JoinHandle<()>)> {
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
    return file_watcher_threads;
}

fn create_log_events_thread(
    notify_rx: Receiver<Result<notify::Event, notify::Error>>,
    log_path: PathBuf,
) {
    let mut file_logger = FileLogger::new(log_path);
    spawn(move || loop {
        match notify_rx.recv() {
            Ok(Ok(file_event)) => file_event.log(&mut file_logger).expect("log event failed"),
            Ok(Err(e)) => println!("notify error: {:?}!", e),
            Err(e) => {
                println!("rx error: {:?}!", e);
                break;
            }
        }
    });
}

pub struct FileGatherer {
    file_watcher_threads: Vec<(Sender<bool>, JoinHandle<()>)>,
}

impl FileGatherer {
    pub fn new(file_paths: Vec<String>, log_path: &str) -> Self {
        let log_path: PathBuf = PathBuf::from(log_path).join("files.json");
        let (notify_tx, notify_rx) = create_notify_channel();
        let file_watcher_threads = create_file_watchers(file_paths, notify_tx);
        create_log_events_thread(notify_rx, log_path);
        Self {
            file_watcher_threads,
        }
    }

    pub fn close(self) {
        for (thread_ctrl, watcher_thread) in self.file_watcher_threads.into_iter() {
            thread_ctrl.send(true).expect("send failed");
            watcher_thread.join().unwrap();
        }
    }
}
