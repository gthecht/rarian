use crate::cacher::{Cache, FileCacher, LoadFromCache};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};
use ulid::Ulid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum NoteStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub link: String,
    pub weight: usize,
}

impl Link {
    pub fn new(link: String, weight: usize) -> Link {
        Link { link, weight }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    id: Ulid,
    links: Vec<Link>,
    pub text: String,
    creation_date: SystemTime,
    status: NoteStatus,
}

impl Note {
    pub fn new(text: &str, links: Vec<Link>) -> Self {
        Self {
            id: Ulid::new(),
            links,
            text: text.to_string(),
            creation_date: SystemTime::now(),
            status: NoteStatus::Active,
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

    pub fn add_note(&mut self, text: &str, links: Vec<String>) {
        let links = links.into_iter().map(|l| Link::new(l, 1)).collect();
        let note = Note::new(text, links);
        self.cacher.cache(&note).expect("cache event failed");
        self.notes.push(note);
    }

    pub fn get_app_notes(&self, link: &str) -> Vec<Note> {
        self.notes
            .iter()
            .filter(|note| {
                (note.links.iter().any(|l| l.link == link)) && (note.status == NoteStatus::Active)
            })
            .rev()
            .cloned()
            .collect()
    }

    pub fn archive_note(&self, link: &str) {
        todo!()
    }

    pub fn edit_note(&self, link: &str) {
        todo!()
    }
}
