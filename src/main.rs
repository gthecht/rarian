mod app;
mod gatherer;
mod notes;

use crate::app::run_app;
use crate::gatherer::app_gatherer::AppGatherer;
use crate::gatherer::file_gatherer::FileGatherer;

fn main() {
    let file_paths = vec![
        "C:/Users/GiladHecht/workspace/rarian".to_string(),
        "D:/Documents/Obsidian".to_string(),
    ];
    let log_path = "C:/Users/GiladHecht/workspace/.rarian/";
    let file_gatherer = FileGatherer::new(file_paths, log_path);
    let app_gatherer = AppGatherer::new(log_path);
    run_app();
    app_gatherer.get_current();
    app_gatherer.get_log();
    app_gatherer.close();
    file_gatherer.close();
}
