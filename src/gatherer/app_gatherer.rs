extern crate sysinfo;
use std::thread::sleep;
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, ProcessExt, System, SystemExt};

fn duration() -> Duration {
    return Duration::from_secs(1);
}

fn init_system() -> System {
    let mut sys = System::new_all();
    sys.refresh_all();
    return sys;
}

pub fn monitor_processes() {
    println!("started system");
    let mut sys = init_system();
    loop {
        sleep(duration());
        sys.refresh_processes_specifics(ProcessRefreshKind::new());
        for (pid, process) in sys.processes() {
            println!("{} {:?}", pid, process.exe());
        }
    }
}