use jsoncodegen_java::codegen;
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

static M2: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = env::home_dir()
        .expect("unable to determine home_dir")
        .join(".m2");
    fs::create_dir_all(&path).expect("Failed to create .m2 directory");
    path
});

#[tokio::test(flavor = "multi_thread")]
async fn test_all() {
    let test_files = fs::read_dir(&*TEST_DATA)
        .expect("Failed to read test-data directory")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "json").then_some(path)
        })
        .collect::<Vec<_>>();

    futures::future::join_all(test_files.iter().map(run_test)).await;
}

async fn run_test(input: &PathBuf) {
    let name = input
        .file_stem()
        .expect("Missing file stem")
        .to_str()
        .expect("Invalid UTF-8 in filename");

    let output = env::temp_dir()
        .join("java")
        .join(input.file_name().expect("Missing file name"));
    let harness = env::temp_dir().join(name);

    fs::create_dir_all(output.parent().expect("Missing parent directory"))
        .expect("Failed to create output directory");
    fs::File::create(&output).expect("Failed to create output file");
    fs::create_dir_all(&harness).expect("Failed to create harness directory");
    copy_dir_all(&*TEMPLATE, &harness).expect("Failed to copy template");

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
