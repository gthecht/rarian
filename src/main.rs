mod gatherer;
use crate::gatherer::file_gatherer::file_gatherer;
use crate::gatherer::app_gatherer::monitor_processes;

fn main() {
    monitor_processes();
    file_gatherer("C:/Users/GiladHecht");
}
