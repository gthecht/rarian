extern crate sysinfo;
use super::logger::{FileLogger, Log, LogEvent};
use active_win_pos_rs::{get_active_window, ActiveWindow};
use anyhow::{Context, Result};
use itertools::Itertools;
use serde::Serialize;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::{Duration, SystemTime};
use sysinfo::{Pid, Process, ProcessExt, ProcessRefreshKind, System, SystemExt};

#[derive(Debug, Clone, Serialize)]
pub struct ActiveProcess {
    title: String,
    app_name: String,
    window_id: String,
    exe: PathBuf,
    process_path: PathBuf,
    process_id: usize,
    parent: Option<usize>,
    start_time: u64,
}

impl ActiveProcess {
    fn new(active_window: ActiveWindow, process: &Process) -> ActiveProcess {
        let title = active_window.title;
        let process_path = active_window.process_path;
        let app_name = active_window.app_name;
        let window_id = active_window.window_id;

        let process_id: usize = active_window
            .process_id
            .try_into()
            .expect("process should fit into usize");
        let parent = process.parent().map(|pid| usize::from(pid));
        let start_time = process.start_time();
        let exe: PathBuf = process.exe().into();
        ActiveProcess {
            title,
            app_name,
            window_id,
            exe,
            process_path,
            process_id,
            parent,
            start_time,
        }
    }
}

impl PartialEq for ActiveProcess {
    fn eq(&self, other: &Self) -> bool {
        self.process_id == other.process_id && self.title == other.title
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveProcessLog {
    process: ActiveProcess,
    active_start_time: SystemTime,
    active_duration: Duration,
}

impl ActiveProcessLog {
    pub fn new(process: ActiveProcess) -> Self {
        ActiveProcessLog {
            process,
            active_start_time: SystemTime::now(),
            active_duration: Duration::new(0, 0),
        }
    }

    pub fn get_title(&self) -> &str {
        &self.process.title
    }
}

impl Display for ActiveProcessLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_title())
    }
}

impl LogEvent<FileLogger> for ActiveProcessLog {
    fn log(&self, file_logger: &mut FileLogger) -> Result<()> {
        let json_string = serde_json::to_string(self).context("json is parsable to string")?;
        file_logger.log(json_string)
    }
}

struct ActiveProcessGatherer {
    current: Arc<Mutex<Option<ActiveProcessLog>>>,
    log: Arc<Mutex<Vec<ActiveProcessLog>>>,
    file_logger: FileLogger,
}

impl ActiveProcessGatherer {
    pub fn new(
        current: Arc<Mutex<Option<ActiveProcessLog>>>,
        log: Arc<Mutex<Vec<ActiveProcessLog>>>,
        file_logger: FileLogger,
    ) -> Self {
        Self {
            current,
            log,
            file_logger,
        }
    }

    pub fn is_current_process(&self, active_process: &ActiveProcess) -> bool {
        let current = &*self.current.lock().unwrap();
        match current {
            Some(current) => current.process == *active_process,
            None => false,
        }
    }

    pub fn update_active_duration(&mut self) {
        let mut current_process = self.current.lock().unwrap();
        if let Some(ref mut current_process) = *current_process {
            current_process.active_duration = SystemTime::now()
                .duration_since(current_process.active_start_time)
                .expect("now is after this process has started");
        }
    }

    pub fn update_current_and_log(&mut self, new_process: ActiveProcessLog) {
        let mut current_process = self.current.lock().unwrap();
        if let Some(ref mut current) = *current_process {
            current
                .log(&mut self.file_logger)
                .expect("log event failed");
            let mut log = self.log.lock().unwrap();
            log.push(current.clone());
        }
        *current_process = Some(new_process);
    }
}

fn duration() -> Duration {
    return Duration::from_secs(1);
}

fn init_system() -> System {
    let mut sys = System::new_all();
    sys.refresh_all();
    return sys;
}

fn get_active_process(sys: &System) -> Option<ActiveProcess> {
    if let Ok(active_window) = get_active_window() {
        let process_id: usize = active_window
            .process_id
            .try_into()
            .expect("process should fit into usize");
        match sys.process(Pid::from(process_id)) {
            Some(active_process) => {
                let active_process = ActiveProcess::new(active_window, active_process);
                return Some(active_process);
            }
            None => panic!("cannot find window process"),
        };
    } else {
        None
    }
}

fn monitor_processes(
    file_logger: FileLogger,
    gatherer_rx: Receiver<bool>,
    current: Arc<Mutex<Option<ActiveProcessLog>>>,
    log: Arc<Mutex<Vec<ActiveProcessLog>>>,
) {
    let mut sys = init_system();
    let mut active_process_gatherer = ActiveProcessGatherer::new(current, log, file_logger);

    while let Err(_) = gatherer_rx.try_recv() {
        sleep(duration());
        sys.refresh_processes_specifics(ProcessRefreshKind::new());

        active_process_gatherer.update_active_duration();
        if let Some(active_process) = get_active_process(&sys) {
            if !active_process_gatherer.is_current_process(&active_process) {
                let new_process = ActiveProcessLog::new(active_process);
                active_process_gatherer.update_current_and_log(new_process);
            }
        } else {
            println!("no active process");
        }
    }
    println!("process monitor stopping gracefully");
}

pub struct AppGatherer {
    thread_ctrl_tx: Sender<bool>,
    gatherer_thread: JoinHandle<()>,
    current: Arc<Mutex<Option<ActiveProcessLog>>>,
    log: Arc<Mutex<Vec<ActiveProcessLog>>>,
}

impl AppGatherer {
    pub fn new(data_path: &Path) -> Self {
        let data_path: PathBuf = PathBuf::from(data_path).join("apps.json");
        let file_logger = FileLogger::new(data_path);
        let (thread_ctrl_tx, thread_ctrl_rx) = channel::<bool>();

        let current = Arc::new(Mutex::new(None));
        let log = Arc::new(Mutex::new(Vec::new()));
        let current_clone = Arc::clone(&current);
        let log_clone = Arc::clone(&log);

        let gatherer_thread =
            spawn(move || monitor_processes(file_logger, thread_ctrl_rx, current_clone, log_clone));
        Self {
            thread_ctrl_tx,
            gatherer_thread,
            current,
            log,
        }
    }

    pub fn get_current(&self) -> Option<ActiveProcessLog> {
        let current = &*self.current.lock().unwrap();
        return current.clone();
    }

    pub fn get_last_processes(&self, n: usize) -> Vec<ActiveProcessLog> {
        let log = &*self.log.lock().unwrap();
        let num = std::cmp::min(n, log.len());
        let last_processes: Vec<ActiveProcessLog> = log
            .iter()
            .rev()
            .unique_by(|app| app.get_title())
            .take(num)
            .map(|process| process.clone())
            .collect();
        return last_processes;
    }

    pub fn close(self) {
        self.thread_ctrl_tx.send(true).expect("send failed");
        self.gatherer_thread.join().unwrap();
    }
}
