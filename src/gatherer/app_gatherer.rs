extern crate sysinfo;
use active_win_pos_rs::{get_active_window, ActiveWindow};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sysinfo::{Pid, ProcessExt, ProcessRefreshKind, System, SystemExt};

#[derive(Debug, Clone)]
struct ActiveProcess {
    active_window: ActiveWindow,
    exe: PathBuf,
    process_id: usize,
    parent: Option<usize>,
    start_time: u64,
}

impl PartialEq for ActiveProcess {
    fn eq(&self, other: &Self) -> bool {
        self.process_id == other.process_id
            && self.active_window == other.active_window
            && self.active_window.title == other.active_window.title
    }
}

#[derive(Debug, Clone)]
struct ActiveProcessLog {
    process: ActiveProcess,
    active_start_time: SystemTime,
    active_duration: Duration,
}

fn duration() -> Duration {
    return Duration::from_secs(1);
}

fn duration_since_epoch(sys_time: SystemTime) -> Duration {
    sys_time
        .duration_since(UNIX_EPOCH)
        .expect("system_time is smaller than unix epoch")
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
                    let parent = active_process.parent().map(|pid| usize::from(pid));
                    let start_time = active_process.start_time();
                    let exe: PathBuf = active_process.exe().into();
                    let active_process = ActiveProcess {
                        active_window,
                        exe,
                        process_id,
                        parent,
                        start_time,
                    };
                    return Some(active_process);
                }
                None => panic!("cannot find window process"),
            };
        }
        Err(()) => None,
    }
}

pub fn monitor_processes() {
    let mut sys = init_system();
    let mut active_process_log: Vec<ActiveProcessLog> = Vec::new();

    loop {
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
                        println!(
                            "{} \nafter {:?} seconds after starting at {:?}",
                            current_process.process.active_window.title,
                            current_process.active_duration,
                            duration_since_epoch(current_process.active_start_time)
                        );
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
}
