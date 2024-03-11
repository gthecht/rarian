mod app;
mod gatherer;
mod notes;

use crate::app::run_app;
use crate::gatherer::app_gatherer::app_gatherer_thread;
use crate::gatherer::file_gatherer::FileGatherer;

fn main() {
    let file_paths = vec![
        "C:/Users/GiladHecht/workspace/rarian".to_string(),
        "D:/Documents/Obsidian".to_string(),
    ];
    let log_path = "C:/Users/GiladHecht/workspace/.rarian/";
    let file_gatherer = FileGatherer::new(file_paths, log_path);
    let cleanup_app_gatherer = app_gatherer_thread(log_path);
    run_app();
    cleanup_app_gatherer();
    file_gatherer.close();
}
