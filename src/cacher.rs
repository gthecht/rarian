use anyhow::{Context, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

pub trait Cache {
    fn cache(&mut self, line: String) -> Result<()>;
}

pub trait CacherLoad {
    fn load_cache(&mut self) -> Result<Vec<String>>;
}

pub trait CacheEvent<Cacher: Cache> {
    fn cache(&self, cacher: &mut Cacher) -> Result<()>;
}

pub trait LoadFromCache<Cacher: Cache> {
    fn deserialize_self(input: &str) -> Result<Self>
    where
        Self: Sized;

    fn load_from_cache(cacher: &mut Cacher) -> Vec<Result<Self>>
    where
        Self: Sized;
}

pub struct FileCacher {
    file: File,
}

impl FileCacher {
    pub fn new(path: PathBuf) -> FileCacher {
        let try_create_file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path);
        match try_create_file {
            Ok(file) => FileCacher { file },
            Err(err) => panic!("Error creating cache file: {:?}", err),
        }
    }
}

impl Cache for FileCacher {
    fn cache(&mut self, line: String) -> Result<()> {
        self.file
            .write_all((line + "\n").as_bytes())
            .context("failed to write line to file")
    }
}

impl CacherLoad for FileCacher {
    fn load_cache(&mut self) -> Result<Vec<String>> {
        let mut read_buffer = String::new();
        let _read_result = self
            .file
            .read_to_string(&mut read_buffer)
            .expect("failed to read file contents");
        Ok(read_buffer.lines().map(String::from).collect())
    }
}
