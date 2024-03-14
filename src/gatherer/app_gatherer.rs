extern crate sysinfo;
use super::logger::{FileLogger, Log, LogEvent};
use active_win_pos_rs::{get_active_window, ActiveWindow};
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::thread::{sleep, spawn};
use std::time::{Duration, SystemTime};
use sysinfo::{Pid, Process, ProcessExt, ProcessRefreshKind, System, SystemExt};

#[derive(Debug, Clone, Serialize)]
struct ActiveProcess {
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
struct ActiveProcessLog {
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
}

impl LogEvent<FileLogger> for ActiveProcessLog {
    fn log(&self, file_logger: &mut FileLogger) -> Result<()> {
        let json_string = serde_json::to_string(self).context("json is parsable to string")?;
        file_logger.log(json_string)
    }
}

struct ActiveProcessGatherer {
    current: Option<ActiveProcessLog>,
    log: Vec<ActiveProcessLog>,
    file_logger: FileLogger,
}

impl ActiveProcessGatherer {
    pub fn new(file_logger: FileLogger) -> Self {
        Self {
            current: None,
            log: Vec::new(),
            file_logger,
        }
    }

    pub fn is_current_process(&self, active_process: &ActiveProcess) -> bool {
        match &self.current {
            Some(current) => current.process == *active_process,
            None => false,
        }
    }

    pub fn update_active_duration(&mut self) {
        if let Some(ref mut current) = self.current {
            (*current).active_duration = SystemTime::now()
                .duration_since(current.active_start_time)
                .expect("now is after this process has started");
        }
    }

    pub fn update_current(&mut self, new_process: ActiveProcessLog) {
        if let Some(ref current) = self.current {
            current
                .log(&mut self.file_logger)
                .expect("log event failed");
            self.log.push(current.clone());
        }
        self.current = Some(new_process);
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
    match get_active_window() {
        Ok(active_window) => {
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
        }
        Err(()) => None,
    }
}

fn monitor_processes(file_logger: FileLogger, gatherer_rx: Receiver<bool>) {
    let mut sys = init_system();
    let mut active_process_gatherer = ActiveProcessGatherer::new(file_logger);

    while let Err(_) = gatherer_rx.try_recv() {
        sleep(duration());
        sys.refresh_processes_specifics(ProcessRefreshKind::new());
        
        active_process_gatherer.update_active_duration();
        if let Some(active_process) = get_active_process(&sys) {
            if !active_process_gatherer.is_current_process(&active_process) {
                let new_process = ActiveProcessLog::new(active_process);
                active_process_gatherer.update_current(new_process);
            }
        } else {
            println!("no active process");
        }
    }
    println!("process monitor stopping gracefully");
}

pub fn app_gatherer_thread(log_path: &str) -> impl FnOnce() {
    let log_path: PathBuf = PathBuf::from(log_path).join("apps.json");
    let file_logger = FileLogger::new(log_path);
    let (gatherer_tx, gatherer_rx) = channel::<bool>();

    let process_monitor_thread_handle = spawn(move || monitor_processes(file_logger, gatherer_rx));
    return move || {
        gatherer_tx
            .send(true)
            .expect("monitor thread should be running");
        process_monitor_thread_handle.join().unwrap();
    };
}
