use crate::gatherer::app_gatherer::ActiveProcessLog;

pub fn new_note(note: &str, process: &ActiveProcessLog) {
    println!("linked to: \n{}", process);
    println!("you wrote down:\n{}", note);
}
