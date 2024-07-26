use crate::cacher::{Cache, FileCacher, LoadFromCache};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
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
    pub id: Ulid,
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
    notes: HashMap<Ulid, Note>,
}

impl NoteTaker {
    pub fn new(data_path: &Path) -> Self {
        let data_path: PathBuf = PathBuf::from(data_path).join("notes.json");
        let mut cacher = FileCacher::new(data_path);
        let notes_from_cache: Vec<Note> = cacher.load_from_cache();
        let notes: HashMap<Ulid, Note> = notes_from_cache
            .into_iter()
            .map(|note| (note.id, note))
            .collect();
        let note_taker = NoteTaker { cacher, notes };
        return note_taker;
    }

    pub fn add_note(&mut self, text: &str, links: Vec<String>) {
        let links = links.into_iter().map(|l| Link::new(l, 1)).collect();
        let note = Note::new(text, links);
        self.cacher.cache(&note).expect("cache event failed");
        self.notes.insert(note.id, note);
    }

    pub fn get_app_notes(&self, link: &str) -> Vec<Note> {
        let mut notes_vec: Vec<Note> = self
            .notes
            .values()
            .filter(|note| {
                (note.links.iter().any(|l| l.link == link)) && (note.status == NoteStatus::Active)
            })
            .cloned()
            .collect();
        notes_vec.sort_by(|a, b| b.creation_date.cmp(&a.creation_date));
        notes_vec
    }

    pub fn archive_note(&mut self, note_id: &Ulid) {
        match self.notes.get(note_id) {
            Some(note) => {
                let mut note = note.to_owned();
                note.status = NoteStatus::Archived;
                self.cacher.cache(&note).expect("cache event failed");
                self.notes.insert(note.id, note);
            },
            None => println!("could not find note with id {}", note_id)
        }
    }

    pub fn edit_note(&self, link: &str) {
        todo!()
    }
}
