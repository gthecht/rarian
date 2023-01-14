use sysinfo::{ProcessExt, System, SystemExt};


fn get_processes() {
    let s = System::new_all();
    for (pid, process) in s.processes() {
        println!("{} {}", pid, process.name());
    }
}