use futures::StreamExt;
use jsoncodegen_rust::codegen;
use jsoncodegen_test_utils::{collect_test_files, copy_dir_all, json_equiv};
use serde_json::Value;
use tokio::process::Command;

use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[tokio::test]
async fn test_all() {
    let n_parallel = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    futures::stream::iter(collect_test_files().into_iter().map(|input| async move {
        run_test(input).await;
    }))
    .buffer_unordered(n_parallel)
    .for_each(|_| async {})
    .await;
}

async fn run_test<P: AsRef<Path>>(input: P) {
    let input = input.as_ref();
    let name = input
        .file_stem()
        .expect("Missing file stem")
        .to_str()
        .expect("Invalid UTF-8 in filename");

    println!("Running test: {}", name);

    let harness = env::temp_dir().join(format!("rust-{}", name));
    let output = harness.join("output.json");

    // Clean up any previous test run
    let _ = fs::remove_dir_all(&harness);
    fs::create_dir_all(&harness).expect("Failed to create harness directory");
    fs::File::create(&output).expect("Failed to create output file");

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

    // Run in Docker
    let cmd_output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/workspace", harness.display()),
            "-v",
            &format!("{}:/data/input.json:ro", input.display()),
            "-v",
            &format!("{}:/data/output.json", output.display()),
            "-w",
            "/workspace",
            "rust:latest",
            "bash",
            "-lc",
            "set -e; cargo run --quiet < /data/input.json > /data/output.json;",
        ])
        .output()
        .await
        .expect("Failed to run Docker container");

    let generated_code = fs::read_to_string(harness.join("src").join("generated.rs"))
        .unwrap_or_else(|_| "<failed to read>".to_string());
    let input_content =
        fs::read_to_string(input).unwrap_or_else(|_| "<failed to read>".to_string());

    assert!(
        cmd_output.status.success(),
        "Run failed for: {name}\n\n--- input.json ---\n{input_content}\n\n--- generated.rs ---\n{generated_code}\n\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&cmd_output.stdout),
        String::from_utf8_lossy(&cmd_output.stderr)
    );

    // Parse the output
    let output_json: Value =
        serde_json::from_reader(fs::File::open(&output).expect("Failed to open output file"))
            .expect("Failed to parse output JSON");
    let expected_json: Value =
        serde_json::from_reader(fs::File::open(input).expect("Failed to open input file"))
            .expect("Failed to parse expected JSON");

    assert!(
        json_equiv(&output_json, &expected_json),
        "Mismatch for: {name}\n\nExpected:\n{expected_json:#?}\n\nActual:\n{output_json:#?}"
    );

    // Clean up after running test
    let _ = fs::remove_dir_all(&harness);
}
