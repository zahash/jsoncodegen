use serde_json::Value;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};
use tokio::process::Command;

/// Check semantic equivalence of two JSON values.
/// Treats `null` values as equivalent to absent fields in objects.
pub fn json_equiv(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| json_equiv(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            // Get all keys from both objects, excluding keys with null values
            let a_keys: std::collections::HashSet<_> = a
                .iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, _)| k)
                .collect();
            let b_keys: std::collections::HashSet<_> = b
                .iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, _)| k)
                .collect();

            // Keys with non-null values must match
            if a_keys != b_keys {
                return false;
            }

            // All non-null values must be equivalent
            a_keys.iter().all(|k| {
                let a_val = a.get(*k).unwrap();
                let b_val = b.get(*k).unwrap();
                json_equiv(a_val, b_val)
            })
        }
        _ => false,
    }
}

/// Recursively copy a directory and all its contents.
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Collect all JSON test files from the TEST_DATA directory.
pub fn collect_test_files() -> Vec<PathBuf> {
    fs::read_dir(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("Failed to get parent directory of CARGO_MANIFEST_DIR")
            .join("test-data"),
    )
    .expect("Failed to read test-data directory")
    .filter_map(|entry| {
        let path = entry.ok()?.path();
        (path.extension()? == "json").then_some(path)
    })
    .collect()
}

/// Run a command inside a Docker container with standard volume mounts.
///
/// Mounts:
/// - `harness_host` -> `/workspace` (RW)
/// - `input_host` -> `/data/input.json` (RO)
/// - `output_host` -> `/data/output.json` (RW)
/// - `extra_volumes` -> as specified
///
/// Workdir is set to `/workspace`.
pub async fn run_in_docker(
    image: &str,
    harness_host: &Path,
    input_host: &Path,
    output_host: &Path,
    extra_volumes: &[(&Path, &str)],
    command: &str,
) -> io::Result<std::process::Output> {
    let mut cmd = Command::new("docker");
    cmd.args(["run", "--rm"]);

    // Mount harness
    cmd.arg("-v")
        .arg(format!("{}:/workspace", harness_host.display()));

    // Mount input (ro)
    cmd.arg("-v")
        .arg(format!("{}:/data/input.json:ro", input_host.display()));

    // Mount output
    cmd.arg("-v")
        .arg(format!("{}:/data/output.json", output_host.display()));

    // Extra volumes
    for (host, container) in extra_volumes {
        cmd.arg("-v")
            .arg(format!("{}:{}", host.display(), container));
    }

    // Workdir
    cmd.arg("-w").arg("/workspace");

    // Image
    cmd.arg(image);

    // Command (using bash -lc)
    cmd.args(["bash", "-lc", command]);

    cmd.output().await
}
