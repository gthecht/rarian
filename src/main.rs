mod gatherer;

use crate::gatherer::app_gatherer::monitor_processes;
use crate::gatherer::file_gatherer::{file_gatherer, cleanup_file_gatherer};

fn main() {
    let (notify_ctrl_tx, file_gatherer_thread) = file_gatherer("C:/Users/GiladHecht/workspace/rarian".to_string());
    monitor_processes();
    cleanup_file_gatherer(notify_ctrl_tx, file_gatherer_thread);
}
