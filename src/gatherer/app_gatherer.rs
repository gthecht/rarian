extern crate sysinfo;
use active_win_pos_rs::{get_active_window, ActiveWindow};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use sysinfo::{Pid, ProcessExt, ProcessRefreshKind, System, SystemExt};

#[derive(Debug, Clone, Default)]
pub struct ActiveProcess {
    pub active_window: ActiveWindow,
    pub exe: PathBuf,
    pub process_id: usize,
    pub parent: Option<usize>,
    pub start_time: u64,
}

impl PartialEq for ActiveProcess {
    fn eq(&self, other: &Self) -> bool {
        self.process_id == other.process_id && self.active_window == other.active_window
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

pub fn monitor_processes() {
    let mut sys = init_system();
    loop {
        sleep(duration());
        sys.refresh_processes_specifics(ProcessRefreshKind::new());

        match get_active_window() {
            Ok(active_window) => {
                let process_id: usize = active_window
                    .process_id
                    .try_into()
                    .expect("process should fit into usize");
                if let Some(active_process) = sys.process(Pid::from(process_id)) {
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
                    println!("active program: {:#?}", active_process);
                };
            }
            Err(()) => {
                println!("error occurred while getting the active window");
            }
        }
    }
}
