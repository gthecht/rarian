extern crate notify;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{spawn, JoinHandle};

fn watch_dir(
    full_path: PathBuf,
    tx: Sender<Result<notify::Event, notify::Error>>,
) -> notify::RecommendedWatcher {
    let mut watcher =
        RecommendedWatcher::new(tx, Config::default()).expect("watcher creation failed");
    watcher
        .watch(full_path.as_ref(), RecursiveMode::Recursive)
        .expect("watcher watching failed");
    return watcher;
}

pub fn watch_dir_thread(
    path: &Path,
    tx: Sender<Result<notify::Event, notify::Error>>,
    thread_ctrl: Receiver<bool>,
) -> JoinHandle<()> {
    let full_path = path.canonicalize().unwrap();
    let watcher_handle = spawn(move || {
        let _watcher = watch_dir(full_path, tx);
        match thread_ctrl.recv() {
            Ok(event) => match &event {
                true => {
                    println!("watcher stopping gracefully");
                }
                false => {
                    println!("watcher stopping not gracefully");
                }
            },
            Err(e) => {
                println!("watcher thread controller got error: {:?}", e);
            }
        }
        return;
    });
    return watcher_handle;
}

#[cfg(test)]
mod file_dir_test {
    use super::*;
    use crate::gatherer::test_utils::test_utils::*;
    use std::fs::create_dir_all;
    use std::sync::mpsc::channel;
    use std::thread::sleep;

    fn create_dir_watcher_and_co(
        test_path: &PathBuf,
    ) -> (
        Receiver<Result<notify::Event, notify::Error>>,
        Sender<bool>,
        JoinHandle<()>,
    ) {
        let (tx, rx) = create_channel();
        let (thread_ctrl, thread_rx) = channel();
        let watcher_thread = watch_dir_thread(test_path.as_path(), tx, thread_rx);
        sleep(duration());
        return (rx, thread_ctrl, watcher_thread);
    }

    fn cleanup_thread(thread_ctrl: Sender<bool>, watcher_thread: JoinHandle<()>) {
        thread_ctrl.send(true).expect("send failed");
        watcher_thread.join().unwrap();
    }

    #[test]
    fn watcher_should_get_path() {
        let (test_path, rmdir_thread) = create_test_dir("watcher_should_get_path");
        let _watcher = watch_dir(
            test_path.as_path().canonicalize().unwrap(),
            create_channel().0,
        );
        rmdir_thread.join().unwrap();
    }

    #[test]
    fn watcher_should_return_correct_event_for_file_in_watched_dir() {
        let (test_path, rmdir_thread) =
            create_test_dir("watcher_should_return_correct_event_for_file_in_watched_dir");
        let (tx, rx) = create_channel();
        let _watcher = watch_dir(test_path.as_path().canonicalize().unwrap(), tx);
        sleep(duration());

        let file_path = create_file_in_dir(&test_path, "tmp.txt", "temp");

        wait_for_event(&rx, &file_path);
        remove_file_in_dir(&test_path, "tmp.txt");
        sleep(duration());
        wait_for_event(&rx, &file_path);
        rmdir_thread.join().unwrap();
    }

    #[test]
    fn watch_sub_folder_recursive() {
        let (test_path, rmdir_thread) = create_test_dir("watch_sub_folder_recursive");
        let (tx, rx) = create_channel();
        let _watcher = watch_dir(test_path.as_path().canonicalize().unwrap(), tx);

        let sub_dir_path = create_test_path("watch_sub_folder_recursive/tmp_dir");
        create_dir_all(&sub_dir_path).expect("create dir failed");
        let sub_file_path = create_file_in_dir(&sub_dir_path, "tmp.txt", "temp");

        wait_for_event(&rx, &sub_file_path);

        rmdir_thread.join().unwrap();
    }

    #[test]
    fn watch_dir_thread_events() {
        let (test_path, rmdir_thread) = create_test_dir("watch_dir_thread_events");
        let (rx, thread_ctrl, watcher_thread) = create_dir_watcher_and_co(&test_path);
        sleep(duration());
        let file_path = create_file_in_dir(&test_path, "tmp.txt", "temp");

        wait_for_event(&rx, &file_path);
        cleanup_thread(thread_ctrl, watcher_thread);
        rmdir_thread.join().unwrap();
    }

    #[test]
    fn watch_dir_thread_stops_gracefully() {
        let (test_path, rmdir_thread) = create_test_dir("watch_dir_thread_stops_gracefully");
        let (_rx, thread_ctrl, watcher_thread) = create_dir_watcher_and_co(&test_path);
        thread_ctrl.send(true).expect("send failed");
        watcher_thread.join().unwrap();
        rmdir_thread.join().unwrap();
    }
}
