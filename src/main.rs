use std::fs::create_dir_all;

mod app;
mod gatherer;
mod notes;
mod cacher;

use crate::app::run_app;
use crate::gatherer::app_gatherer::AppGatherer;
use crate::gatherer::file_gatherer::FileGatherer;
use directories::{BaseDirs, ProjectDirs};

fn main() {
    let base_dirs = BaseDirs::new().unwrap();
    let file_paths = vec![
        base_dirs.home_dir().join("workspace"),
    ];
    let project_dir = ProjectDirs::from("", "Rarian", "rarian").unwrap();
    let data_path = project_dir.data_dir();
    create_dir_all(data_path).expect("Creating the project directories in Roaming failed");

    let file_gatherer = FileGatherer::new(file_paths, data_path);
    let app_gatherer = AppGatherer::new(data_path);
    run_app(&app_gatherer, data_path);
    app_gatherer.close();
    file_gatherer.close();
}
