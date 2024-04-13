use serde_json::{self, Value};
extern crate notify;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};
use std::time::SystemTime;

use crate::cacher::{Cache, FileCacher};
use crate::gatherer::file_watcher::watch_dir_thread;

fn cache_event(event: &notify::Event) -> Value {
    let timestamp = SystemTime::now();
    serde_json::json!({
        "event": event,
        "timestamp": timestamp
    })
}

fn create_notify_channel() -> (
    Sender<Result<notify::Event, notify::Error>>,
    Receiver<Result<notify::Event, notify::Error>>,
) {
    return channel();
}

fn create_file_watchers(
    file_paths: Vec<PathBuf>,
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

fn create_caching_thread(
    notify_rx: Receiver<Result<notify::Event, notify::Error>>,
    data_path: PathBuf,
) {
    let mut cacher = FileCacher::new(data_path.clone());
    spawn(move || loop {
        let cache_path = data_path.as_path().to_str().expect("path to string failed");
        match notify_rx.recv() {
            Ok(Ok(file_event)) => {
                let file_paths = file_event
                    .paths
                    .iter()
                    .map(|p| p.to_str().expect("path to string failed"));

                if file_paths.clone().filter(|path| path.ends_with(cache_path)).count() > 0 {
                    continue;
                }
                    cacher
                        .cache(&cache_event(&file_event))
                        .expect("cache event failed");
                if file_paths.filter(|path| path.contains(r"\.")).count() > 0 {
                    continue;
                }
            }
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
    pub fn new(file_paths: Vec<PathBuf>, data_path: &Path) -> Self {
        let data_path: PathBuf = PathBuf::from(data_path).join("files.json");
        let (notify_tx, notify_rx) = create_notify_channel();
        let file_watcher_threads = create_file_watchers(file_paths, notify_tx);
        create_caching_thread(notify_rx, data_path);
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
