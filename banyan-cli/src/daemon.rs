use banyan_guts::{native::NativeError, shared::PID_FILE};
use std::path::Path;
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

pub fn daemon_is_running() -> bool {
    let pid_file_path = Path::new(PID_FILE);
    if pid_file_path.exists() {
        let pid_contents = std::fs::read_to_string(pid_file_path).unwrap();
        let pid = pid_contents.trim().parse::<usize>().unwrap();
        let system = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::new()),
        );
        matches!(system.process(Pid::from(pid)), Some(_process))
    } else {
        false
    }
}

pub fn start_daemon() -> Result<(), NativeError> {
    if !daemon_is_running() {
        let child = std::process::Command::new("banyan-daemon").spawn().unwrap();
        std::fs::write(PID_FILE, child.id().to_string())?;
        Ok(())
    } else {
        Err(NativeError::custom_error("daemon already running"))
    }
}

pub fn stop_daemon() -> Result<String, NativeError> {
    let pid_file_path = Path::new(PID_FILE);
    if pid_file_path.exists() {
        let pid_contents = std::fs::read_to_string(pid_file_path)?;
        let pid = pid_contents.trim().parse::<usize>().unwrap();
        let system = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::new()),
        );
        let process = system
            .process(Pid::from(pid))
            .ok_or(NativeError::custom_error("Daemon not running"))?;
        process.kill();
        Ok("Killed daemon".to_string())
    } else {
        Err(NativeError::custom_error("Daemon not running"))
    }
}
