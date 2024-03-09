mod gatherer;
mod app;

use crate::gatherer::app_gatherer::app_gatherer_thread;
use crate::gatherer::file_gatherer::file_gatherer;
use crate::app::run_app;

fn main() {
    let file_paths = vec![
        "C:/Users/GiladHecht/workspace/rarian".to_string(),
        "D:/Documents/Obsidian".to_string(),
    ];
    let log_path = "C:/Users/GiladHecht/workspace/.rarian/";
    let cleanup_file_gatherer = file_gatherer(file_paths, log_path);
    let cleanup_app_gatherer = app_gatherer_thread(log_path);
    run_app();
    cleanup_app_gatherer();
    cleanup_file_gatherer();
}
