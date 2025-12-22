use jsoncodegen_java::codegen;
use pretty_assertions::assert_eq;

use std::{
    env::{self, temp_dir},
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::LazyLock,
};

static TEST_DATA: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory of CARGO_MANIFEST_DIR")
        .join("test-data")
});

static TEST_RESULTS: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = env::temp_dir().join("java");
    fs::create_dir_all(&path).expect("Failed to create <temp_dir>/java directory");
    path
});

static TEMPLATE: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("template")
});

static M2: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = env::home_dir()
        .expect("unable to determine home_dir")
        .join(".m2");
    fs::create_dir_all(&path).expect("Failed to create .m2 directory");
    path
});

#[test]
fn analytics_events() {
    test("analytics-events")
}

#[test]
fn config_file() {
    test("config-file")
}

#[test]
fn ecommerce_api_response() {
    test("ecommerce-api-response")
}

#[test]
fn linked_list() {
    test("linked-list")
}

#[test]
fn tree() {
    test("tree")
}

#[track_caller]
fn test(test_name: &str) {
    // check if docker exists
    Command::new("docker")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn Docker command");

    let test_input = TEST_DATA.join(format!("{}.json", test_name));
    let test_output = TEST_RESULTS.join(format!("{}.json", test_name));
    fs::File::create(&test_output).expect("Failed to create test output file");

    let harness = temp_dir().join(test_name);
    fs::create_dir_all(&harness).expect("Failed to create harness directory");
    copy_dir_all(&*TEMPLATE, &harness).expect("Failed to copy test harness");

    codegen(
        serde_json::from_reader(
            fs::File::open(&test_input).expect("Failed to open test input file"),
        )
        .expect("Failed to deserialize test input file"),
        &mut fs::File::create(harness.join("src").join("JsonCodeGen.java"))
            .expect("Failed to create JsonCodeGen.java file"),
    )
    .expect("Failed to write to JsonCodeGen.java");

    #[rustfmt::skip]
    let status = Command::new("docker")
        .args([
            "run", "--rm",
            "-v", &format!("{}:/workspace", harness.display()),
            "-v", &format!("{}:/root/.m2", M2.display()),
            "-v", &format!("{}:/data/input.json:ro", test_input.display()),
            "-v", &format!("{}:/data/output.json", test_output.display()),
            "-w", "/workspace",
            "docker.io/library/maven:3.9.9-eclipse-temurin-17",
            "bash", "-lc",
            "   set -e;\
                mvn clean package;\
                mvn dependency:copy-dependencies -DoutputDirectory=target/lib -DincludeScope=runtime;\
                java -jar target/jsoncodegen.jar < /data/input.json > /data/output.json;",
        ])
        .status()
        .expect("Failed to run Docker container");

    assert!(status.success(), "Docker container failed");

    let output_json = serde_json::from_reader::<_, serde_json::Value>(
        fs::File::open(&test_output).expect("Failed to open output JSON file"),
    )
    .expect("Failed to parse output JSON");

    let expected_json = serde_json::from_reader::<_, serde_json::Value>(
        fs::File::open(test_input).expect("Failed to open input JSON file"),
    )
    .expect("Failed to parse input JSON");

    assert_eq!(output_json, expected_json);
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
