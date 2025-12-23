use jsoncodegen_rust::codegen;
use serde_json::Value;
use tokio::process::Command;

use std::{
    env, fs, io,
    path::{Path, PathBuf},
    sync::LazyLock,
};

static TEST_DATA: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory of CARGO_MANIFEST_DIR")
        .join("test-data")
});

static TEMPLATE: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("template")
});

#[tokio::test]
async fn test_all() {
    let test_files = fs::read_dir(&*TEST_DATA)
        .expect("Failed to read test-data directory")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "json").then_some(path)
        })
        .collect::<Vec<_>>();

    for input in test_files {
        run_test(&input).await;
    }
}

async fn run_test(input: &PathBuf) {
    let name = input
        .file_stem()
        .expect("Missing file stem")
        .to_str()
        .expect("Invalid UTF-8 in filename");

    println!("Running test: {}", name);

    let harness = env::temp_dir().join(format!("rust-{}", name));

    // Clean up any previous test run
    let _ = fs::remove_dir_all(&harness);
    fs::create_dir_all(&harness).expect("Failed to create harness directory");
    copy_dir_all(&*TEMPLATE, &harness).expect("Failed to copy template");

    // Generate the Rust code
    codegen(
        serde_json::from_reader(fs::File::open(input).expect("Failed to open input file"))
            .expect("Failed to parse input JSON"),
        &mut fs::File::create(harness.join("src").join("generated.rs"))
            .expect("Failed to create generated.rs"),
    )
    .expect("Failed to run codegen");

    // Build the test project
    let build_output = Command::new("cargo")
        .args(["build"])
        .current_dir(&harness)
        .output()
        .await
        .expect("Failed to run cargo build");

    let generated_code = fs::read_to_string(harness.join("src").join("generated.rs"))
        .unwrap_or_else(|_| "<failed to read>".to_string());
    let input_content =
        fs::read_to_string(input).unwrap_or_else(|_| "<failed to read>".to_string());

    assert!(
        build_output.status.success(),
        "Build failed for: {name}\n\n--- input.json ---\n{input_content}\n\n--- generated.rs ---\n{generated_code}\n\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&build_output.stdout),
        String::from_utf8_lossy(&build_output.stderr)
    );

    // Run the test binary with input JSON
    let run_output = Command::new(harness.join("target").join("debug").join("jsoncodegen"))
        .stdin(fs::File::open(input).expect("Failed to open input file"))
        .output()
        .await
        .expect("Failed to run test binary");

    assert!(
        run_output.status.success(),
        "Run failed for: {name}\n\n--- input.json ---\n{input_content}\n\n--- generated.rs ---\n{generated_code}\n\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );

    // Parse the output
    let output_json: Value =
        serde_json::from_slice(&run_output.stdout).expect("Failed to parse output JSON");
    let expected_json: Value =
        serde_json::from_reader(fs::File::open(input).expect("Failed to open input file"))
            .expect("Failed to parse expected JSON");

    assert!(
        json_equiv(&output_json, &expected_json),
        "Mismatch for: {name}\n\nExpected:\n{expected_json:#?}\n\nActual:\n{output_json:#?}"
    );
}

/// Check semantic equivalence of two JSON values.
/// Treats `null` values as equivalent to absent fields in objects.
fn json_equiv(a: &Value, b: &Value) -> bool {
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

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
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
