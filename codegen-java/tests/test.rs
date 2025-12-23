use jsoncodegen_java::codegen;
use jsoncodegen_test_utils::{collect_test_files, copy_dir_all, json_equiv};
use serde_json::Value;
use tokio::process::Command;

use std::{env, fs, path::PathBuf, sync::LazyLock};

static M2: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = env::home_dir()
        .expect("unable to determine home_dir")
        .join(".m2");
    fs::create_dir_all(&path).expect("Failed to create .m2 directory");
    path
});

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

    let output = env::temp_dir()
        .join("java")
        .join(input.file_name().expect("Missing file name"));
    let harness = env::temp_dir().join(name);

    fs::create_dir_all(output.parent().expect("Missing parent directory"))
        .expect("Failed to create output directory");
    fs::File::create(&output).expect("Failed to create output file");
    fs::create_dir_all(&harness).expect("Failed to create harness directory");
    copy_dir_all(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("template"),
        &harness,
    )
    .expect("Failed to copy template");

    codegen(
        serde_json::from_reader(fs::File::open(input).expect("Failed to open input file"))
            .expect("Failed to parse input JSON"),
        &mut fs::File::create(harness.join("src").join("JsonCodeGen.java"))
            .expect("Failed to create JsonCodeGen.java"),
    )
    .expect("Failed to run codegen");

    #[rustfmt::skip]
    let cmd_output = Command::new("docker")
        .args([
            "run", "--rm",
            "-v", &format!("{}:/workspace", harness.display()),
            "-v", &format!("{}:/root/.m2", M2.display()),
            "-v", &format!("{}:/data/input.json:ro", input.display()),
            "-v", &format!("{}:/data/output.json", output.display()),
            "-w", "/workspace",
            "docker.io/library/maven:3.9.9-eclipse-temurin-17",
            "bash", "-lc",
            "   set -e;\
                mvn clean package;\
                mvn dependency:copy-dependencies -DoutputDirectory=target/lib -DincludeScope=runtime;\
                java -jar target/jsoncodegen.jar < /data/input.json > /data/output.json;",
        ])
        .output()
        .await
        .expect("Failed to run Docker container");

    let generated_code = fs::read_to_string(harness.join("src").join("JsonCodeGen.java"))
        .unwrap_or_else(|_| "<failed to read>".to_string());
    let input_content =
        fs::read_to_string(input).unwrap_or_else(|_| "<failed to read>".to_string());

    assert!(
        cmd_output.status.success(),
        "Docker failed for: {name}\n\n--- input.json ---\n{input_content}\n\n--- JsonCodeGen.java ---\n{generated_code}\n\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&cmd_output.stdout),
        String::from_utf8_lossy(&cmd_output.stderr)
    );

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
}
