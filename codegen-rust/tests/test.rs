use jsoncodegen_rust::codegen;
use jsoncodegen_test_utils::{collect_test_files, copy_dir_all, json_equiv};
use serde_json::Value;
use tokio::process::Command;

use std::{env, fs, path::PathBuf};

#[tokio::test]
async fn test_all() {
    for input in collect_test_files() {
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
    copy_dir_all(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("template"),
        &harness,
    )
    .expect("Failed to copy template");

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

    // Clean up after running test
    let _ = fs::remove_dir_all(&harness);

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
