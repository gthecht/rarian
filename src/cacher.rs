use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use anyhow::{Context, Result};

pub trait Cache {
    fn cache(&mut self, line: String) -> Result<()>;
}

pub trait CacheEvent<Cacher: Cache> {
    fn cache(&self, cacher: &mut Cacher) -> Result<()>;
}

pub struct FileCacher {
    file: File,
}

impl FileCacher {
    pub fn new(path: PathBuf) -> FileCacher {
        let try_create_file = OpenOptions::new().append(true).create(true).open(path);
        match try_create_file {
            Ok(file) => FileCacher { file },
            Err(err) => panic!("Error creating cache file: {:?}", err),
        }
    }
}

impl Cache for FileCacher {
    fn cache(&mut self, line: String) ->Result<()> {
        self.file.write_all((line + "\n").as_bytes()).context("failed to write line to file")
    }
}
