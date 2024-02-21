mod gatherer;

use crate::gatherer::app_gatherer::monitor_processes;
use crate::gatherer::file_gatherer::file_gatherer;

fn main() {
    let file_paths = vec![
        "C:/Users/GiladHecht/workspace/rarian".to_string(),
        "D:/Documents/Obsidian".to_string(),
    ];
    let cleanup_file_gatherer = file_gatherer(file_paths);
    monitor_processes();
    cleanup_file_gatherer();
}
