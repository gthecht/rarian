use std::{
    sync::mpsc::{channel, Sender},
    thread::spawn,
};
mod app;
mod cacher;
mod config;
mod gatherer;
mod notes;

use crate::app::tui::run_app;
use crate::gatherer::app_gatherer::AppGatherer;
use crate::gatherer::file_gatherer::FileGatherer;
use config::Config;
use gatherer::app_gatherer::ActiveProcessEvent;
use notes::{Note, NoteTaker};
use ulid::Ulid;

pub enum Signals {
    RecentApps(usize, Sender<Vec<ActiveProcessEvent>>),
    CurrentApp(Sender<Option<ActiveProcessEvent>>),
    GetLinkNotes(String, Sender<(Vec<Note>, Vec<Note>, Vec<Note>)>),
    NewNote(String, Vec<String>),
    ArchiveNote(Ulid),
    EditNote(Ulid, String),
    Quit,
}

fn main() {
    change_window_title();
    let config = Config::new();
    let app_gatherer = AppGatherer::new(&config);
    let mut note_taker = NoteTaker::new(config.data_path.as_path());
    let (action_tx, action_rx) = channel::<Signals>();
    let file_gatherer = FileGatherer::new(action_tx.clone(), &config);
    let app_thread = spawn(move || {
        run_app(config, action_tx.clone());
    });

    use Signals::*;
    loop {
        match action_rx.recv() {
            Ok(RecentApps(n, tx)) => {
                let _ = tx.send(app_gatherer.get_last_processes(n));
            }
            Ok(CurrentApp(tx)) => {
                let _ = tx.send(app_gatherer.get_current());
            }
            Ok(GetLinkNotes(link, tx)) => {
                let _ = tx.send(note_taker.get_linked_notes(&link));
            }
            Ok(NewNote(text, links)) => note_taker.add_note(&text, links),
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
