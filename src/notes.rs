use crate::cacher::{Cache, FileCacher, LoadFromCache};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    link: String,
    pub text: String,
}

impl Note {
    pub fn new(text: &str, link: &str) -> Self {
        Self {
            link: link.to_string(),
            text: text.to_string(),
        }
    }
}

pub struct NoteTaker {
    cacher: FileCacher,
    notes: Vec<Note>,
}

impl NoteTaker {
    pub fn new(data_path: &Path) -> Self {
        let data_path: PathBuf = PathBuf::from(data_path).join("notes.json");
        let mut cacher = FileCacher::new(data_path);
        let notes: Vec<Note> = cacher.load_from_cache();
        let note_taker = NoteTaker { cacher, notes };
        return note_taker;
    }

    pub fn add_note(&mut self, text: &str, link: &str) {
        let note = Note::new(text, link);
        self.cacher.cache(&note).expect("cache event failed");
        self.notes.push(note);
    }

    pub fn get_app_notes(&self, link: &str) -> Vec<Note> {
        self.notes
            .iter()
            .filter(|note| note.link == link)
            .rev()
            .cloned()
            .collect()
    }
}
