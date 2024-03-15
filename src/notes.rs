use crate::gatherer::{
    app_gatherer::ActiveProcessLog,
    logger::{FileLogger, Log, LogEvent},
};
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct Note {
    process: String,
    text: String,
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
    pub fn new(log_path: &str) -> Self {
        let log_path: PathBuf = PathBuf::from(log_path).join("notes.json");
        let file_logger = FileLogger::new(log_path);
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
}
