use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;
mod gatherer;
use crate::gatherer::file_watcher::watch_dir_thread;

fn create_channel() -> (
    Sender<Result<notify::Event, notify::Error>>,
    Receiver<Result<notify::Event, notify::Error>>,
) {
    return channel();
}

fn cleanup_thread(thread_ctrl: Sender<bool>, watcher_thread: JoinHandle<()>) {
    thread_ctrl.send(true).expect("send failed");
    watcher_thread.join().unwrap();
}

fn main() {
    let (tx, rx) = create_channel();
    let (thread_ctrl, thread_rx) = channel();
    let path: PathBuf = PathBuf::from("./".to_owned());
    let watcher_thread = watch_dir_thread(path.as_path(), tx, thread_rx);

    loop {
        match rx.recv() {
            Ok(event) => {
                println!("Hello, {:?}", event);
            }
            Err(e) => {
                println!("rx error: {:?}!", e);
                break;
            }
        }
    }

    cleanup_thread(thread_ctrl, watcher_thread);
}
