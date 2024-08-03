use anyhow;
use clap::Parser;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::str;
use std::time::Duration;
use std::{
    fs::{create_dir_all, File},
    io::BufReader,
    path::{Path, PathBuf},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    data_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub data_path: PathBuf,
    pub watcher_paths: Vec<PathBuf>,
    pub comment_identifier: String,
    pub sleep_duration: Duration,
}

impl Config {
    pub fn new() -> Config {
        let args = Args::parse();
        let data_path = args.data_path.unwrap_or_else(|| {
            let project_dir = ProjectDirs::from("", "Rarian", "rarian").unwrap();
            project_dir.data_dir().to_path_buf()
        });
        create_dir_all(&data_path).expect("Creating the project directories in Roaming failed");
        let config_path = data_path.join("config.json");
        match Self::read_config_from_file(config_path) {
            Ok(config) => config,
            Err(_) => {
                println!("failed to load config file");
                let comment_identifier = vec!["@", "#", "$"].join("");
                let sleep_duration = Duration::from_millis(16);
                Config {
                    data_path: data_path.to_path_buf(),
                    watcher_paths: vec![],
                    comment_identifier,
                    sleep_duration,
                }
            }
        }
    }

    fn read_config_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let config = serde_json::from_reader(reader)?;
        Ok(config)
    }
}
