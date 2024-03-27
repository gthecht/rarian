use crate::cacher::{Cache, CacheEvent, CacherLoad, FileCacher, LoadFromCache};
use crate::gatherer::app_gatherer::ActiveProcessEvent;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
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

impl LoadFromCache<FileCacher> for Note {
    fn deserialize_self(input: &str) -> Result<Self> {
        serde_json::from_str::<Self>(&input).context("Failed to parse note")
    }

    fn load_from_cache(cacher: &mut FileCacher) -> Vec<Result<Self>> {
        match cacher.load_cache() {
            Ok(json_string) => {
                let notes = json_string
                    .into_iter()
                    .map(|note_string| Self::deserialize_self(&note_string))
                    .collect();
                return notes;
            }
            Err(_) => Vec::new(),
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
        let notes = Note::load_from_cache(&mut cacher);
        let notes: Vec<Note> = notes
            .into_iter()
            .filter_map(|n_result| n_result.ok())
            .collect();
        let note_taker = NoteTaker { cacher, notes };
        return note_taker;
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
