use fclones::{
    log::{Log, LogLevel, ProgressBarLength},
    progress::ProgressTracker,
};
use log::{debug, error, warn};
use std::sync::Arc;

/// A custom logger for fclones that uses the normal-person logging crate
#[derive(Debug, Default)]
pub struct FClonesLogger {}

impl Log for FClonesLogger {
    fn progress_bar(&self, _msg: &str, _len: ProgressBarLength) -> Arc<dyn ProgressTracker> {
        Arc::new(fclones::progress::NoProgressBar)
    }
    fn log(&self, level: LogLevel, msg: String) {
        match level {
            LogLevel::Info => debug!("fclones says: {}", msg),
            LogLevel::Warn => warn!("fclones says: {}", msg),
            LogLevel::Error => error!("fclones says: {}", msg),
        }
    }
}
