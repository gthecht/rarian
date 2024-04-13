use std::fs::create_dir_all;

mod app;
mod gatherer;
mod notes;
mod cacher;

use crate::app::run_app;
use crate::gatherer::app_gatherer::AppGatherer;
use crate::gatherer::file_gatherer::FileGatherer;
use directories::{BaseDirs, ProjectDirs};
use notes::NoteTaker;

fn main() {
    let base_dirs = BaseDirs::new().unwrap();
    let file_paths = vec![
        base_dirs.home_dir().join("workspace"),
    ];
    let project_dir = ProjectDirs::from("", "Rarian", "rarian").unwrap();
    let data_path = project_dir.data_dir();
    create_dir_all(data_path).expect("Creating the project directories in Roaming failed");

    let app_gatherer = AppGatherer::new(data_path);
    let mut note_taker = NoteTaker::new(data_path);
    let file_gatherer = FileGatherer::new(&app_gatherer, &mut note_taker, file_paths, data_path);
    run_app(&app_gatherer, &mut note_taker);
    app_gatherer.close();
    file_gatherer.close();
}
