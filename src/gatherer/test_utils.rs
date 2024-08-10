#[cfg(test)]
pub mod test_utils {
    use std::fs::{create_dir_all, remove_dir_all, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::channel;
    use std::sync::mpsc::{Receiver, Sender};
    use std::thread::sleep;
    use std::thread::{spawn, JoinHandle};
    use std::time::Duration;

    pub fn duration() -> Duration {
        return Duration::from_millis(10);
    }

    fn long_duration() -> Duration {
        return 20 * duration();
    }

    pub fn create_test_path(test_id: &str) -> PathBuf {
        let testdata_path = Path::new("./testData");
        testdata_path.join(test_id)
    }

    pub fn create_test_dir(test_id: &str) -> (PathBuf, JoinHandle<()>) {
        let path_buf = create_test_path(test_id);
        println!("{:?}", path_buf);
        create_dir_all(&path_buf).expect("create dir failed");
        let rmdir_thread = cleanup_test_path_delayed(&path_buf, None);
        return (path_buf, rmdir_thread);
    }

    fn cleanup_test_path(path_buf: &PathBuf) {
        remove_dir_all(path_buf).expect("remove dir failed");
    }

    fn cleanup_test_path_delayed(
        path_buf: &PathBuf,
        test_duration: Option<Duration>,
    ) -> JoinHandle<()> {
        let clean_path = path_buf.clone();
        return spawn(move || {
            sleep(test_duration.unwrap_or(long_duration()));
            cleanup_test_path(&clean_path);
        });
    }

    fn create_file_path(path: &PathBuf, filename: &str) -> PathBuf {
        let mut file_path = PathBuf::new();
        file_path.push(path);
        file_path.push(filename);
        return file_path;
    }

    pub fn create_channel() -> (
        Sender<Result<notify::Event, notify::Error>>,
        Receiver<Result<notify::Event, notify::Error>>,
    ) {
        return channel();
    }

    pub fn wait_for_event(
        rx: &Receiver<Result<notify::Event, notify::Error>>,
        file_path: &PathBuf,
    ) {
        let event = rx.recv_timeout(long_duration());
        match event {
            Ok(rx_result) => match rx_result {
                Ok(event_result) => {
                    let first_path = event_result.paths.get(0);
                    if first_path == Some(file_path) {
                        return;
                    } else {
                        return wait_for_event(rx, file_path);
                    }
                }
                Err(e) => panic!("{}", e),
            },
            Err(e) => panic!("{}", e),
        }
    }

    pub fn create_file_in_dir(path: &PathBuf, filename: &str, contents: &str) -> PathBuf {
        let file_path = create_file_path(path, filename);
        let mut file_ref = File::create(&file_path).expect("create failed");
        file_ref.write(contents.as_bytes()).expect("write failed");
        return file_path.canonicalize().unwrap();
    }

    pub fn remove_file_in_dir(path: &PathBuf, filename: &str) {
        let mut file_path = PathBuf::new();
        file_path.push(path);
        file_path.push(filename);
        std::fs::remove_file(file_path).expect("remove failed");
    }
}
