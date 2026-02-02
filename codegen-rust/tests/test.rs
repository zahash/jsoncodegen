use jsoncodegen_rust::codegen;
use jsoncodegen_test_macro::generate_tests;
use jsoncodegen_test_utils::{copy_dir_all, json_equiv};
use serde_json::Value;
use tokio::process::Command;

use std::{
    env, fs,
    path::{Path, PathBuf},
};

generate_tests!("test-data");

async fn run_test<P: AsRef<Path>>(input_filepath: P) {
    let input_filepath = input_filepath.as_ref();
    let name = input_filepath
        .file_stem()
        .expect("Missing file stem")
        .to_str()
        .expect("Invalid UTF-8 in filename");

    let workspace_dir = env::temp_dir().join(env!("CARGO_PKG_NAME")).join(name);
    fs::create_dir_all(&workspace_dir).expect(&format!(
        "Failed to create workspace directory :: {:?}",
        workspace_dir
    ));

    copy_dir_all(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("template"),
        &workspace_dir,
    )
    .expect("Failed to copy template");

    let output_filepath = workspace_dir.join("output.json");
    fs::File::create(&output_filepath).expect("Failed to create output file");

    codegen(
        serde_json::from_reader(fs::File::open(input_filepath).expect("Failed to open input file"))
            .expect("Failed to parse input JSON"),
        &mut fs::File::create(workspace_dir.join("src").join("generated.rs"))
            .expect("Failed to create generated.rs"),
    )
    .expect("Failed to run codegen");

    // TODO: have a common target dir mounted so redundant compilations can be avoided.
    //       set CARGO_TARGET_DIR env var to some path and mount it
    #[rustfmt::skip]
    let cmd_output = Command::new("docker")
        .args([
            "run", "--rm",
            "-v", &format!("{}:/workspace", workspace_dir.display()),
            "-v", &format!("{}:/data/input.json:ro", input_filepath.display()),
            "-v", &format!("{}:/data/output.json", output_filepath.display()),
            "-w", "/workspace",
            "docker.io/library/rust:slim",
            "bash", "-lc",
            "    set -e;\
                 /usr/local/cargo/bin/cargo run --quiet < /data/input.json > /data/output.json;",
        ])
        .output()
        .await
        .expect("Failed to run Docker container");

    let generated_code = fs::read_to_string(workspace_dir.join("src").join("generated.rs"))
        .unwrap_or_else(|_| "<failed to read>".to_string());
    let input_content =
        fs::read_to_string(input_filepath).unwrap_or_else(|_| "<failed to read>".to_string());

    assert!(
        cmd_output.status.success(),
        "Run failed for: {name}\n\n--- input.json ---\n{input_content}\n\n--- generated.rs ---\n{generated_code}\n\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&cmd_output.stdout),
        String::from_utf8_lossy(&cmd_output.stderr)
    );

    let output_json: Value = serde_json::from_reader(
        fs::File::open(&output_filepath).expect("Failed to open output file"),
    )
    .expect("Failed to parse output JSON");
    let expected_json: Value =
        serde_json::from_reader(fs::File::open(input_filepath).expect("Failed to open input file"))
            .expect("Failed to parse expected JSON");

    assert!(
        json_equiv(&output_json, &expected_json),
        "Mismatch for: {name}\n\nExpected:\n{expected_json:#?}\n\nActual:\n{output_json:#?}"
    );

    // TODO: workspace_dir doesn't get removed if there is a panic in above lines of code
    if let Err(e) = fs::remove_dir_all(workspace_dir) {
        eprintln!("{:?}", e);
    }
}
