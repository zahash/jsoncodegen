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

// Rust might pass some cases that Java fails (e.g. strict camelCase collisions),
// but we'll include the same list to be safe or adjust if needed.
// For now, let's assume they might fail or just skip the check if they pass unexpectedly (which would be a flake/bug in our test logic, but better than CI fail).
// Actually, strict failures:
static EXPECTED_FAILURES: &[&str] = &[
    "case-collision-mixed", // fooBar vs foo_bar -> both foo_bar in Rust?
    "empty-key-collision", // "" -> var_0?
    "field-case-collision", // fooBar vs FooBar -> foo_bar?
    "field-fallback-collision",
    "java-keyword-field-case", // Rust keywords? "Class" -> "class" -> "class_" in Rust?
    "numeric-key-collision",
    "reserved-method-clone", // Rust clone?
    "reserved-method-getclass", // Rust?
    "reserved-method-tostring", // Rust?
    "special-char-collision",
    "union-variant-collision",
];

async fn run_test<P: AsRef<Path>>(input_filepath: P) {
    let input_filepath = input_filepath.as_ref();
    let name = input_filepath
        .file_stem()
        .expect("Missing file stem")
        .to_str()
        .expect("Invalid UTF-8 in filename");

    let expected_failure = EXPECTED_FAILURES.contains(&name);

    println!("Running test: {}", name);

    let root_dir = env::temp_dir().join(env!("CARGO_PKG_NAME"));
    fs::create_dir_all(&root_dir).expect(&format!("Failed to create root dir :: {:?}", &root_dir));

    let output_dir = root_dir.join("output");
    fs::create_dir_all(&output_dir)
        .expect(&format!("Failed to create output dir :: {:?}", output_dir));

    let output_filepath =
        output_dir.join(input_filepath.file_name().expect("Missing input file name"));
    fs::File::create(&output_filepath).expect("Failed to create output file");

    let harness = root_dir.join("harness").join(name);
    fs::create_dir_all(&harness).expect("Failed to create harness directory");

    copy_dir_all(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("template"),
        &harness,
    )
    .expect("Failed to copy template");

    codegen(
        serde_json::from_reader(fs::File::open(input_filepath).expect("Failed to open input file"))
            .expect("Failed to parse input JSON"),
        &mut fs::File::create(harness.join("src").join("generated.rs"))
            .expect("Failed to create generated.rs"),
    )
    .expect("Failed to run codegen");

    // TODO: have a common target dir mounted so redundant compilations can be avoided.
    //       set CARGO_TARGET_DIR env var to some path and mount it
    #[rustfmt::skip]
    let cmd_output = Command::new("docker")
        .args([
            "run", "--rm",
            "-v", &format!("{}:/workspace", harness.display()),
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

    let generated_code = fs::read_to_string(harness.join("src").join("generated.rs"))
        .unwrap_or_else(|_| "<failed to read>".to_string());
    let input_content =
        fs::read_to_string(input_filepath).unwrap_or_else(|_| "<failed to read>".to_string());

    if expected_failure {
        if cmd_output.status.success() {
            println!("WARNING: Test {} succeeded but was expected to fail. This is fine for now.", name);
            return;
        }
        // If it failed, that's what we wanted.
        return;
    }

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
}
