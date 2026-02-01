use futures::StreamExt;
use jsoncodegen_java::codegen;
use jsoncodegen_test_utils::{collect_test_files, copy_dir_all, json_equiv};
use serde_json::Value;
use tokio::process::Command;

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};

static M2: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = env::home_dir()
        .expect("unable to determine home_dir")
        .join(".m2");
    fs::create_dir_all(&path).expect("Failed to create .m2 directory");
    path
});

static EXPECTED_FAILURES: &[&str] = &[
    "case-collision-mixed",
    "empty-key-collision",
    "field-case-collision",
    "field-fallback-collision",
    "java-keyword-field-case",
    "numeric-key-collision",
    "reserved-method-clone",
    "reserved-method-getclass",
    "reserved-method-tostring",
    "special-char-collision",
    "union-variant-collision",
];

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

    let harness_dir = root_dir.join("harness").join(name);
    fs::create_dir_all(&harness_dir).expect("Failed to create harness directory");

    copy_dir_all(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("template"),
        &harness_dir,
    )
    .expect("Failed to copy template");

    codegen(
        serde_json::from_reader(fs::File::open(input_filepath).expect("Failed to open input file"))
            .expect("Failed to parse input JSON"),
        &mut fs::File::create(harness_dir.join("src").join("JsonCodeGen.java"))
            .expect("Failed to create JsonCodeGen.java"),
    )
    .expect("Failed to run codegen");

    #[rustfmt::skip]
    let cmd_output = Command::new("docker")
        .args([
            "run", "--rm",
            "-v", &format!("{}:/workspace", harness_dir.display()),
            "-v", &format!("{}:/root/.m2", M2.display()),
            "-v", &format!("{}:/data/input.json:ro", input_filepath.display()),
            "-v", &format!("{}:/data/output.json", output_filepath.display()),
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

    let generated_code = fs::read_to_string(harness_dir.join("src").join("JsonCodeGen.java"))
        .unwrap_or_else(|_| "<failed to read>".to_string());
    let input_content =
        fs::read_to_string(input_filepath).unwrap_or_else(|_| "<failed to read>".to_string());

    if expected_failure {
        assert!(
            !cmd_output.status.success(),
            "Docker succeeded but expected failure for: {name}\n\n--- input.json ---\n{input_content}\n\n--- JsonCodeGen.java ---\n{generated_code}\n\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&cmd_output.stdout),
            String::from_utf8_lossy(&cmd_output.stderr)
        );
        return;
    }

    assert!(
        cmd_output.status.success(),
        "Docker failed for: {name}\n\n--- input.json ---\n{input_content}\n\n--- JsonCodeGen.java ---\n{generated_code}\n\n--- stdout ---\n{}\n--- stderr ---\n{}",
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
