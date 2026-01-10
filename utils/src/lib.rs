use std::{env, path::PathBuf};

pub fn default_runtime_dir() -> PathBuf {
    let mut runtime_dir = env::home_dir()
        .or_else(|| env::current_dir().ok())
        .unwrap_or_default();
    runtime_dir.push(".jsoncodegen");
    runtime_dir
}
