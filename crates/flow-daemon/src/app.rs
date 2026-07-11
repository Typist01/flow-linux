use flow_config::Config;
use fs4::fs_std::FileExt;
use std::fs::{File, OpenOptions};
use std::io;
use tracing_subscriber::EnvFilter;

pub fn init_logging(config: &Config) {
    let level = config.general.log_level.as_str();
    let filter = EnvFilter::try_new(level)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .init();
}

pub struct InstanceLock {
    _file: File,
}

pub fn acquire_single_instance() -> io::Result<InstanceLock> {
    let lock_path = Config::lock_path();
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)?;

    file.try_lock_exclusive().map_err(|e| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "another flow-daemon is already running (lock: {}) — {}",
                lock_path.display(),
                e
            ),
        )
    })?;

    tracing::info!(path = %lock_path.display(), "acquired single-instance lock");
    Ok(InstanceLock { _file: file })
}
