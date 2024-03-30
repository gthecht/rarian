use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

pub trait Cache<T>
where
    T: Serialize,
{
    fn cache(&mut self, obj: &T) -> Result<()>;
}

pub trait LoadFromCache<T>
where
    T: Sized,
{
    fn load_from_cache(&mut self) -> Vec<T>;
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

impl<T> Cache<T> for FileCacher
where
    T: Serialize + Sized,
{
    fn cache(&mut self, obj: &T) -> Result<()> {
        let line = serde_json::to_string(&obj).expect("Serialization failed");
        self.file
            .write_all((line + "\n").as_bytes())
            .context("failed to write line to file")
    }
}

impl<T> LoadFromCache<T> for FileCacher
where
    T: for<'a> Deserialize<'a>,
{
    fn load_from_cache(&mut self) -> Vec<T> {
        let mut read_buffer = String::new();
        match self.file.read_to_string(&mut read_buffer) {
            Ok(_) => read_buffer
                .lines()
                .filter_map(|line| {
                    serde_json::from_str::<T>(&line)
                        .context("Failed to parse line")
                        .ok()
                })
                .collect(),
            Err(e) => {
                println!("Error reading from cache: {}", e);
                Vec::new()
            }
        }
    }
}
