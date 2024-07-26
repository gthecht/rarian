use itertools::Itertools;
use notify::event::ModifyKind;
use notify::EventKind;
use serde_json::{self, Value};
use std::str;
extern crate notify;
use anyhow::Result;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};
use std::time::SystemTime;

use crate::cacher::{Cache, FileCacher};
use crate::gatherer::app_gatherer::ActiveProcessEvent;
use crate::gatherer::file_watcher::watch_dir_thread;
use crate::StateMachine;

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

fn check_for_notes(state_machine_tx: Sender<StateMachine>, file_event: notify::Event) {
    file_event.paths.iter().for_each(|path| {
        if path.is_file() {
            let mut file = OpenOptions::new()
                .read(true)
                .open(path)
                .expect("is openable file");
            let mut read_buffer = String::new();
            match file.read_to_string(&mut read_buffer) {
                Ok(_) => {
                    let comment_identifier = str::from_utf8(&[64, 35, 36]).unwrap(); // wrote the string in bytes so that they won't be misinterpreted as part of a note 😅
                    let split_file: Vec<&str> =
                        read_buffer.trim().split(comment_identifier).collect();
                    if split_file.len() > 1 && split_file.len() % 2 == 1 {
                        let mut file_text = Vec::<&str>::new();
                        let mut notes = Vec::<&str>::new();
                        split_file
                            .into_iter()
                            .enumerate()
                            .for_each(|(index, text)| {
                                if index % 2 == 0 {
                                    file_text.push(text);
                                } else {
                                    notes.push(text);
                                }
                            });
                        let (tx, rx) = channel::<Option<ActiveProcessEvent>>();
                        state_machine_tx.send(StateMachine::CurrentApp(tx)).unwrap();
                        let process = rx
                            .recv()
                            .expect("main thread is alive")
                            .expect("there must be a process for file changes");
                        let mut new_file_text = file_text.into_iter().map(|text| text).join("");
                        new_file_text.push_str("\n");
                        if let Ok(mut write_file) =
                            OpenOptions::new().truncate(true).write(true).open(path)
                        {
                            if let Ok(_) = write_file.write_all(new_file_text.as_bytes()) {
                                notes.into_iter().for_each(|note| {
                                    let mut links: Vec<String> = file_event
                                        .paths
                                        .iter()
                                        .map(|p| p.to_string_lossy().to_string())
                                        .collect();
                                    links.push(process.get_title().to_string());
                                    state_machine_tx
                                        .send(StateMachine::NewNote(note.trim().to_string(), links))
                                        .unwrap();
                                })
                            } else {
                                println!("failed to write to file will try again next time");
                            }
                        } else {
                            println!("Error opening file for writing: {:?}", path)
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {}
                Err(err) => {
                    panic!("failed to parse file: {}", err)
                }
            }
        }
    });
}

fn act_on_event(state_machine_tx: Sender<StateMachine>, file_event: notify::Event) {
    match file_event.kind {
        EventKind::Modify(ModifyKind::Any | ModifyKind::Data(_) | ModifyKind::Other) => {
            check_for_notes(state_machine_tx, file_event)
        }
        _ => {}
    }
}

fn create_caching_thread(
    state_machine_tx: Sender<StateMachine>,
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

                if file_paths
                    .clone()
                    .filter(|path| path.ends_with(cache_path))
                    .count()
                    > 0
                {
                    continue;
                }
                cacher
                    .cache(&cache_event(&file_event))
                    .expect("cache event failed");
                if file_paths.filter(|path| path.contains(r"\.")).count() > 0 {
                    continue;
                }
                act_on_event(state_machine_tx.clone(), file_event);
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
    pub fn new(
        state_machine_tx: Sender<StateMachine>,
        file_paths: Vec<PathBuf>,
        data_path: &Path,
    ) -> Self {
        let data_path: PathBuf = PathBuf::from(data_path).join("files.json");
        let (notify_tx, notify_rx) = create_notify_channel();
        let file_watcher_threads = create_file_watchers(file_paths, notify_tx);
        create_caching_thread(state_machine_tx, notify_rx, data_path);
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
