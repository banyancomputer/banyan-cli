use crate::service::start_service;
use banyan_guts::native::NativeError;
use daemonize::Daemonize;
use std::fs::File;
use std::path::Path;
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

pub const PID_FILE: &str = "/var/run/banyan-daemon.pid";
const STDOUT: &str = "/var/log/banyan-daemon.out";
const STDERR: &str = "/var/log/banyan-daemon.err";

pub fn build_daemonize() -> Result<String, NativeError> {
    let stdout_file = File::create(STDOUT).expect("Failed to create stdout file");
    let stderr_file = File::create(STDERR).expect("Failed to create stderr file");
    let daemonize = Daemonize::new()
        .pid_file(PID_FILE)
        .working_directory("/tmp")
        .user("nobody")
        .group("daemon")
        .stdout(stdout_file)
        .stderr(stderr_file)
        .privileged_action(|| async { start_service().await });
    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error, {}", e),
    }
    Ok("Started daemon".to_string())
}

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

pub fn kill_daemon() -> Result<String, NativeError> {
    let pid_file_path = Path::new(PID_FILE);
    if !pid_file_path.exists() {
        return Err(NativeError::daemon_error(
            "Daemon is not running".to_string(),
        ));
    }
    let pid_contents = std::fs::read_to_string(pid_file_path).unwrap();
    let pid = pid_contents.trim().parse::<usize>().unwrap();
    let system =
        System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new()));
    let process = system.process(Pid::from(pid)).unwrap();
    process.kill();
    Ok("Killed daemon".to_string())
}
