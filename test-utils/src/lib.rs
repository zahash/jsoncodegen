use serde_json::Value;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

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
