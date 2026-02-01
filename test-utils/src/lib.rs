use serde_json::Value;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};
use tokio::process::Command;

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

pub struct TestConfig {
    /// The language name (used for logging and directory naming)
    pub language: &'static str,
    /// Path to the test input JSON file
    pub input_file: PathBuf,
    /// Path to the template directory to copy
    pub template_dir: PathBuf,
    /// Function to generate code from JSON input
    pub codegen_fn: Box<dyn Fn(Value, &mut fs::File) -> io::Result<()> + Send + Sync>,
    /// Docker image to run
    pub docker_image: &'static str,
    /// Command to run inside the Docker container
    pub docker_command: &'static str,
    /// Extra volumes to mount (host path, container path, options)
    pub extra_volumes: Vec<(PathBuf, &'static str, &'static str)>,
    /// Working directory inside the container
    pub work_dir: &'static str,
    /// Relative path to the output source file within the harness (e.g., "src/generated.rs")
    pub source_path: &'static str,
}

pub async fn run_test(config: TestConfig) {
    let input_filepath = config.input_file.as_path();
    let name = input_filepath
        .file_stem()
        .expect("Missing file stem")
        .to_str()
        .expect("Invalid UTF-8 in filename");

    println!("Running test: {} ({})", name, config.language);

    // Setup directories
    let root_dir = env::temp_dir().join(format!("jsoncodegen-{}", config.language));
    fs::create_dir_all(&root_dir).expect(&format!("Failed to create root dir :: {:?}", &root_dir));

    let output_dir = root_dir.join("output");
    fs::create_dir_all(&output_dir)
        .expect(&format!("Failed to create output dir :: {:?}", output_dir));

    let output_filepath = output_dir.join(
        input_filepath
            .file_name()
            .expect("Missing input file name"),
    );
    fs::File::create(&output_filepath).expect("Failed to create output file");

    let harness_dir = root_dir.join("harness").join(name);
    // Clean up previous harness if it exists to avoid side effects
    if harness_dir.exists() {
        fs::remove_dir_all(&harness_dir).expect("Failed to remove existing harness directory");
    }
    fs::create_dir_all(&harness_dir).expect("Failed to create harness directory");

    // Copy template
    copy_dir_all(&config.template_dir, &harness_dir).expect("Failed to copy template");

    // Run codegen
    let input_json: Value = serde_json::from_reader(
        fs::File::open(input_filepath).expect("Failed to open input file"),
    )
    .expect("Failed to parse input JSON");

    let source_file_path = harness_dir.join(config.source_path);
    if let Some(parent) = source_file_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create source file directory");
    }

    (config.codegen_fn)(
        input_json.clone(),
        &mut fs::File::create(&source_file_path).expect("Failed to create source file"),
    )
    .expect("Failed to run codegen");

    // Build Docker arguments
    let mut args = vec![
        "run".to_string(),
        "--rm".to_string(),
        "-v".to_string(),
        format!("{}:{}", harness_dir.display(), config.work_dir),
        "-v".to_string(),
        format!("{}:/data/input.json:ro", input_filepath.display()),
        "-v".to_string(),
        format!("{}:/data/output.json", output_filepath.display()),
    ];

    for (host_path, container_path, options) in config.extra_volumes {
        args.push("-v".to_string());
        let volume_spec = if options.is_empty() {
             format!("{}:{}", host_path.display(), container_path)
        } else {
             format!("{}:{}:{}", host_path.display(), container_path, options)
        };
        args.push(volume_spec);
    }

    args.extend_from_slice(&[
        "-w".to_string(),
        config.work_dir.to_string(),
        config.docker_image.to_string(),
        "bash".to_string(),
        "-lc".to_string(),
        config.docker_command.to_string(),
    ]);

    // Run Docker
    let cmd_output = Command::new("docker")
        .args(&args)
        .output()
        .await
        .expect("Failed to run Docker container");

    let generated_code = fs::read_to_string(&source_file_path)
        .unwrap_or_else(|_| "<failed to read>".to_string());
    let input_content =
        fs::read_to_string(input_filepath).unwrap_or_else(|_| "<failed to read>".to_string());

    assert!(
        cmd_output.status.success(),
        "Docker run failed for: {name}\n\n--- input.json ---\n{input_content}\n\n--- Generated Code ---\n{generated_code}\n\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&cmd_output.stdout),
        String::from_utf8_lossy(&cmd_output.stderr)
    );

    // Verify output
    let output_json: Value = serde_json::from_reader(
        fs::File::open(&output_filepath).expect("Failed to open output file"),
    )
    .expect("Failed to parse output JSON");

    assert!(
        json_equiv(&output_json, &input_json),
        "Mismatch for: {name}\n\nExpected:\n{input_json:#?}\n\nActual:\n{output_json:#?}"
    );
}
