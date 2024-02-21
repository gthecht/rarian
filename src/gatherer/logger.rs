use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use anyhow::{Context, Result};

pub trait Log {
    fn log(&mut self, msg: String) -> Result<()>;
}

pub trait LogEvent<Logger: Log> {
    fn log_event(&self, logger: &mut Logger) -> Result<()>;
}

pub struct FileLogger {
    file: File,
}

impl FileLogger {
    pub fn new(path: PathBuf) -> FileLogger {
        let try_create_file = OpenOptions::new().write(true).append(true).open(path);
        match try_create_file {
            Ok(file) => FileLogger { file },
            Err(err) => panic!("Error creating log file: {:?}", err),
        }
    }
}

impl Log for FileLogger {
    fn log(&mut self, msg: String) ->Result<()> {
        self.file.write_all((msg + ",\n").as_bytes()).context("failed to write msg to file")
    }
}
