use std::{
    sync::mpsc::{channel, Sender},
    thread::spawn,
};

mod app;
mod cacher;
mod gatherer;
mod notes;
mod config;

use crate::app::tui::run_app;
use crate::gatherer::app_gatherer::AppGatherer;
use crate::gatherer::file_gatherer::FileGatherer;
use config::Config;
use gatherer::app_gatherer::ActiveProcessEvent;
use notes::{Note, NoteTaker};
use ulid::Ulid;

pub enum StateMachine {
    RecentApps(usize, Sender<Vec<ActiveProcessEvent>>),
    CurrentApp(Sender<Option<ActiveProcessEvent>>),
    GetAppNotes(String, Sender<Vec<Note>>),
    NewNote(String, Vec<String>),
    ArchiveNote(Ulid),
    EditNote(Ulid, String),
    Quit,
}

fn main() {
    change_window_title();
    let config = Config::new();
    let app_gatherer = AppGatherer::new(config.data_path.as_path());
    let mut note_taker = NoteTaker::new(config.data_path.as_path());
    let (action_tx, action_rx) = channel::<StateMachine>();
    let file_gatherer = FileGatherer::new(
        action_tx.clone(),
        config.watcher_paths,
        config.data_path.as_path(),
    );
    let app_thread = spawn(move || {
        run_app(action_tx.clone());
    });

    use StateMachine::*;
    loop {
        match action_rx.recv() {
            Ok(RecentApps(n, tx)) => {
                let _ = tx.send(app_gatherer.get_last_processes(n));
            }
            Ok(CurrentApp(tx)) => {
                let _ = tx.send(app_gatherer.get_current());
            }
            Ok(GetAppNotes(link, tx)) => {
                let _ = tx.send(note_taker.get_app_notes(&link));
            }
            Ok(NewNote(text, links)) => {
                note_taker.add_note(&text, links);
            }
            Ok(ArchiveNote(note_id)) => note_taker.archive_note(&note_id),
            Ok(EditNote(note_id, text)) => note_taker.edit_note(&note_id, &text),
            Ok(Quit) => break,
            Err(err) => {
                println!("action error: {}", err);
            }
        }
    }
    app_gatherer.close();
    file_gatherer.close();
    app_thread.join().unwrap();
}

fn change_window_title() {
    print!("\x1b]0;Rarian app\x07");
    // Flush the output to ensure the title is set immediately
    use std::io::{self, Write};
    io::stdout().flush().unwrap();
}
