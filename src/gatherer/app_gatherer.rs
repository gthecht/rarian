extern crate sysinfo;
use anyhow::{Context, Result};
use serde::Serialize;
use super::logger::{FileLogger, Log, LogEvent};
use active_win_pos_rs::{get_active_window, ActiveWindow};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::thread::{sleep, spawn};
use std::time::{Duration, SystemTime};
use sysinfo::{Pid, Process, ProcessExt, ProcessRefreshKind, System, SystemExt};

#[derive(Debug, Clone, Serialize)]
struct ActiveProcess {
    title: String,
    process_path: PathBuf,
    app_name: String,
    window_id: String,
    exe: PathBuf,
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
            process_path,
            app_name,
            window_id,
            exe,
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

impl LogEvent<FileLogger> for ActiveProcessLog {
    fn log_event(&self, file_logger: &mut FileLogger) -> Result<()> {
        let json_string = serde_json::to_string(self).context("json is parsable to string")?;
        file_logger.log(json_string)
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

fn monitor_processes(mut file_logger: FileLogger, gatherer_rx: Receiver<bool>) {
    let mut sys = init_system();
    let mut active_process_log: Vec<ActiveProcessLog> = Vec::new();

    while let Err(_) = gatherer_rx.try_recv() {
        sleep(duration());
        sys.refresh_processes_specifics(ProcessRefreshKind::new());

        let active_process = get_active_process(&sys);
        match active_process {
            Some(active_process) => {
                if let Some(current_process) = active_process_log.last_mut() {
                    if current_process.process == active_process {
                        current_process.active_duration = SystemTime::now()
                            .duration_since(current_process.active_start_time)
                            .expect("now is after this process has started");
                    } else {
                        current_process.active_duration = SystemTime::now()
                            .duration_since(current_process.active_start_time)
                            .expect("now is after this process has started");
                        current_process
                            .log_event(&mut file_logger)
                            .expect("log event failed");

                        let new_process = ActiveProcessLog {
                            process: active_process,
                            active_start_time: SystemTime::now(),
                            active_duration: Duration::new(0, 0),
                        };
                        active_process_log.push(new_process);
                    }
                } else {
                    let new_process = ActiveProcessLog {
                        process: active_process,
                        active_start_time: SystemTime::now(),
                        active_duration: Duration::new(0, 0),
                    };
                    active_process_log.push(new_process);
                }
            }
            None => println!("no active process"),
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
        gatherer_tx.send(true).expect("monitor thread should be running");
        process_monitor_thread_handle.join().unwrap();
    };
}
