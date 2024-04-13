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
use crate::gatherer::file_watcher::watch_dir_thread;
use crate::notes::Note;

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

fn check_for_notes(file_event: notify::Event) {
    let notes: Vec<Result<Option<Vec<Note>>, &str>> = file_event
        .paths
        .iter()
        .map(|path| {
            if let Ok(mut file) = OpenOptions::new().read(true).open(path) {
                let mut read_buffer = String::new();
                match file.read_to_string(&mut read_buffer) {
                    Ok(_) => {
                        let comment_identifier = str::from_utf8(&[64, 35, 36]).unwrap();
                        let split_file: Vec<&str> =
                            read_buffer.trim().split(comment_identifier).collect();
                        if split_file.len() > 1 && split_file.len() % 2 == 1 {
                            println!("identified note!");
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
                            let notes: Vec<Note> =
                                notes
                                    .into_iter()
                                    .map(|note| {
                                        Note::new(note.trim(), path.to_str().expect(
                                        "path conversion to string was already run before this",
                                    ))
                                    })
                                    .collect();
                            let mut new_file_text =
                                file_text.into_iter().map(|text| text.trim_end()).join("");
                            new_file_text.push_str("\n");
                            if let Ok(mut write_file) =
                                OpenOptions::new().truncate(true).write(true).open(path)
                            {
                                if let Ok(_) = write_file.write_all(new_file_text.as_bytes()) {
                                    println!("successfully extracted notes");
                                    return Ok(Some(notes));
                                } else {
                                    return Err("failed to write to file");
                                }
                            } else {
                                Err("Error reading file")
                            }
                        } else {
                            if split_file.len() == 1 {
                                println!("no comment identifiers found");
                            } else {
                                println!("odd number of comment identifiers");
                            }
                            return Ok(None);
                        }
                    }
                    Err(_) => Err("Error parsing file part to string"),
                }
            } else {
                Err("Error reading file")
            }
        })
        .collect();
    notes.iter().for_each(|note| println!("{:?}", note));
}

fn act_on_event(file_event: notify::Event) {
    match file_event.kind {
        EventKind::Modify(ModifyKind::Any | ModifyKind::Data(_) | ModifyKind::Other) => {
            check_for_notes(file_event)
        }
        _ => {}
    }
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
                act_on_event(file_event);
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
