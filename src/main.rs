mod gatherer;

use crate::gatherer::app_gatherer::monitor_processes;
use crate::gatherer::file_gatherer::file_gatherer;

fn main() {
    let file_paths = vec![
        "C:/Users/GiladHecht/workspace/rarian".to_string(),
        "D:/Documents/Obsidian".to_string(),
    ];
    let log_path = "C:/Users/GiladHecht/workspace/.rarian/";
    let cleanup_file_gatherer = file_gatherer(file_paths, log_path);
    monitor_processes(log_path);
    cleanup_file_gatherer();
}
