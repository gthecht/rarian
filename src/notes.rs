use crate::cacher::{Cache, CacheEvent, FileCacher};
use crate::gatherer::app_gatherer::ActiveProcessEvent;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct Note {
    process: String,
    pub text: String,
}

impl Note {
    pub fn new(text: &str, process: &ActiveProcessEvent) -> Self {
        Self {
            process: process.get_title().to_string(),
            text: text.to_string(),
        }
    }
}

impl CacheEvent<FileCacher> for Note {
    fn cache(&self, cacher: &mut FileCacher) -> Result<()> {
        let json_string = serde_json::to_string(self).context("json is parsable to string")?;
        cacher.cache(json_string)
    }
}

pub struct NoteTaker {
    cacher: FileCacher,
    notes: Vec<Note>,
}

impl NoteTaker {
    pub fn new(data_path: &Path) -> Self {
        let data_path: PathBuf = PathBuf::from(data_path).join("notes.json");
        let cacher = FileCacher::new(data_path);
        NoteTaker {
            cacher,
            notes: Vec::new(),
        }
    }

    pub fn add_note(&mut self, text: &str, process: &ActiveProcessEvent) {
        let note = Note::new(text, process);
        note.cache(&mut self.cacher).expect("cache event failed");
        self.notes.push(note);
    }

    pub fn get_app_notes(&self, process_title: &str) -> Vec<&Note> {
        self.notes
            .iter()
            .filter(|note| note.process == process_title)
            .collect()
    }
}
