use crate::gatherer::{
    app_gatherer::ActiveProcessLog,
    logger::{FileLogger, Log, LogEvent},
};
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct Note {
    process: String,
    pub text: String,
}

impl Note {
    pub fn new(text: &str, process: &ActiveProcessLog) -> Self {
        Self {
            process: process.get_title().to_string(),
            text: text.to_string(),
        }
    }
}

impl LogEvent<FileLogger> for Note {
    fn log(&self, file_logger: &mut FileLogger) -> Result<()> {
        let json_string = serde_json::to_string(self).context("json is parsable to string")?;
        file_logger.log(json_string)
    }
}

pub struct NoteTaker {
    file_logger: FileLogger,
    notes: Vec<Note>,
}

impl NoteTaker {
    pub fn new(data_path: &Path) -> Self {
        let data_path: PathBuf = PathBuf::from(data_path).join("notes.json");
        let file_logger = FileLogger::new(data_path);
        NoteTaker {
            file_logger,
            notes: Vec::new(),
        }
    }

    pub fn add_note(&mut self, text: &str, process: &ActiveProcessLog) {
        let note = Note::new(text, process);
        note.log(&mut self.file_logger).expect("log event failed");
        self.notes.push(note);
    }

    pub fn get_app_notes(&self, process_title: &str) -> Vec<&Note> {
        self.notes
            .iter()
            .filter(|note| note.process == process_title)
            .collect()
    }
}
