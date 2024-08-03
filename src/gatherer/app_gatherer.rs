extern crate sysinfo;
use crate::cacher::{Cache, FileCacher, LoadFromCache};
use crate::config::Config;
use active_win_pos_rs::{get_active_window, ActiveWindow};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::{Duration, SystemTime};
use sysinfo::{Pid, Process, ProcessExt, ProcessRefreshKind, System, SystemExt};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        let app_name = active_window.app_name;
        let mut title = active_window.title.trim_start_matches("● ").to_string();
        if title == "" {
            title = app_name.clone();
        }
        let process_path = active_window.process_path;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProcessEvent {
    process: ActiveProcess,
    active_start_time: SystemTime,
    active_duration: Duration,
}

impl ActiveProcessEvent {
    pub fn new(process: ActiveProcess) -> Self {
        ActiveProcessEvent {
            process,
            active_start_time: SystemTime::now(),
            active_duration: Duration::new(0, 0),
        }
    }

    pub fn get_title(&self) -> &str {
        &self.process.title
    }
}

impl Display for ActiveProcessEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_title())
    }
}

struct ActiveProcessGatherer {
    current: Arc<Mutex<Option<ActiveProcessEvent>>>,
    process_events: Arc<Mutex<Vec<ActiveProcessEvent>>>,
    cacher: FileCacher,
}

impl ActiveProcessGatherer {
    pub fn new(
        current: Arc<Mutex<Option<ActiveProcessEvent>>>,
        process_events: Arc<Mutex<Vec<ActiveProcessEvent>>>,
        cacher: FileCacher,
    ) -> Self {
        Self {
            current,
            process_events,
            cacher,
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

    pub fn update_current_and_cache(&mut self, new_process: Option<ActiveProcessEvent>) {
        let mut current_process = self.current.lock().unwrap();
        if let Some(ref mut current) = *current_process {
            self.cacher.cache(current).expect("cache event failed");
            let mut process_events = self.process_events.lock().unwrap();
            process_events.push(current.clone());
        }
        *current_process = new_process;
    }
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
    cacher: FileCacher,
    sleep_duration: Duration,
    gatherer_rx: Receiver<bool>,
    current: Arc<Mutex<Option<ActiveProcessEvent>>>,
    process_events: Arc<Mutex<Vec<ActiveProcessEvent>>>,
) {
    let mut sys = init_system();
    let mut active_process_gatherer = ActiveProcessGatherer::new(current, process_events, cacher);

    const IGNORE_APPS: [&str; 3] = ["Rarian app", "Task Switching", ""];
    while let Err(_) = gatherer_rx.try_recv() {
        sleep(sleep_duration);
        sys.refresh_processes_specifics(ProcessRefreshKind::new());

        active_process_gatherer.update_active_duration();
        match get_active_process(&sys) {
            Some(active_process) => {
                if !active_process_gatherer.is_current_process(&active_process) {
                    if IGNORE_APPS.contains(&active_process.title.as_str()) {
                        continue;
                    }
                    let new_process = ActiveProcessEvent::new(active_process);
                    active_process_gatherer.update_current_and_cache(Some(new_process));
                }
            }
            None => {
                active_process_gatherer.update_current_and_cache(None);
            }
        }
    }
    println!("process monitor stopping gracefully");
}

pub struct AppGatherer {
    thread_ctrl_tx: Sender<bool>,
    gatherer_thread: JoinHandle<()>,
    current: Arc<Mutex<Option<ActiveProcessEvent>>>,
    process_events: Arc<Mutex<Vec<ActiveProcessEvent>>>,
}

impl AppGatherer {
    pub fn new(config: &Config) -> Self {
        let data_path: PathBuf = PathBuf::from(config.data_path.clone()).join("apps.json");
        let mut cacher = FileCacher::new(data_path);
        let sleep_duration = config.sleep_duration;

        let (thread_ctrl_tx, thread_ctrl_rx) = channel::<bool>();
        let current = Arc::new(Mutex::new(None));
        let process_events = Arc::new(Mutex::new(cacher.load_from_cache()));
        let current_clone = Arc::clone(&current);
        let process_events_clone = Arc::clone(&process_events);

        let gatherer_thread = spawn(move || {
            monitor_processes(cacher, sleep_duration, thread_ctrl_rx, current_clone, process_events_clone)
        });
        Self {
            thread_ctrl_tx,
            gatherer_thread,
            current,
            process_events,
        }
    }

    pub fn get_current(&self) -> Option<ActiveProcessEvent> {
        let current = &*self.current.lock().unwrap();
        return current.clone();
    }

    pub fn get_last_processes(&self, n: usize) -> Vec<ActiveProcessEvent> {
        let process_events = &*self.process_events.lock().unwrap();
        let num = std::cmp::min(n, process_events.len());
        let last_processes: Vec<ActiveProcessEvent> = process_events
            .iter()
            .rev()
            .unique_by(|app| app.get_title())
            .take(num)
            .map(|process| process.clone())
            .collect();
        last_processes
    }

    pub fn close(self) {
        self.thread_ctrl_tx.send(true).expect("send failed");
        self.gatherer_thread.join().unwrap();
    }
}
