use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::HashSet,
    env, fs, io,
    path::{Path, PathBuf},
};
use tokio::process::Command;

#[derive(Deserialize, Debug)]
pub struct Manifest {
    pub template: Template,
    pub docker: Docker,
}

#[derive(Deserialize, Debug)]
pub struct Template {
    pub dir: PathBuf,
    pub codegen_output: PathBuf,
}

#[derive(Deserialize, Debug)]
pub struct Docker {
    pub image: String,
    pub mounts: Vec<Mount>,
    pub script: PathBuf,
}

#[derive(Deserialize, Debug)]
pub struct Mount {
    pub source: PathBuf,
    pub target: PathBuf,
}

pub async fn test<F, P>(manifest: &Manifest, codegen: F, input_filepath: P)
where
    F: FnOnce(Value, &mut dyn io::Write) -> io::Result<()>,
    P: AsRef<Path>,
{
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

    copy_dir_all(&manifest.template.dir, &workspace_dir).expect(&format!(
        "Failed to copy template :: {}",
        manifest.template.dir.display()
    ));

    let output_filepath = workspace_dir.join("output.json");
    fs::File::create(&output_filepath).expect("Failed to create output file");

    let codegen_output_filepath = workspace_dir.join(&manifest.template.codegen_output);
    codegen(
        serde_json::from_reader(fs::File::open(input_filepath).expect(&format!(
            "Failed to open input file :: {}",
            input_filepath.display()
        )))
        .expect("Failed to parse input JSON"),
        &mut fs::File::create(&codegen_output_filepath).expect(&format!(
            "Failed to create file :: {}",
            codegen_output_filepath.display()
        )),
    )
    .expect("Failed to run codegen");

    const CNT_INPUT: &str = "/input.json";
    const CNT_OUTPUT: &str = "/output.json";
    const CNT_SCRIPT: &str = "/script.sh";

    let mut command = Command::new("docker");

    #[rustfmt::skip]
    command
        .args([
            "run", "--rm",
            "-v", &format!("{}:/workspace", workspace_dir.display()),
            "-v", &format!("{}:{}:ro", input_filepath.display(), CNT_INPUT),
            "-v", &format!("{}:{}", output_filepath.display(), CNT_OUTPUT),
            "-v", &format!("{}:{}:ro", &manifest.docker.script.display(), CNT_SCRIPT),
            ]);

    for mount in &manifest.docker.mounts {
        command.args([
            "-v",
            &format!("{}:{}", mount.source.display(), mount.target.display()),
        ]);
    }

    #[rustfmt::skip]
    command.args([
        "-w", "/workspace",
        &&manifest.docker.image,
    ]);

    command.arg("bash");
    command.arg(CNT_SCRIPT);
    command.args([CNT_INPUT, CNT_OUTPUT]);

    let cmd_output = command
        .output()
        .await
        .expect("Failed to run Docker container");

    let generated_code = fs::read_to_string(&codegen_output_filepath)
        .unwrap_or_else(|_| "<failed to read>".to_string());
    let input_content =
        fs::read_to_string(input_filepath).unwrap_or_else(|_| "<failed to read>".to_string());

    assert!(
        cmd_output.status.success(),
        "Run failed for: {name}\n\n--- input.json ---\n{input_content}\n\n--- {} ---\n{generated_code}\n\n--- stdout ---\n{}\n--- stderr ---\n{}",
        codegen_output_filepath.display(),
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
            let a_keys: HashSet<_> = a
                .iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, _)| k)
                .collect();
            let b_keys: HashSet<_> = b
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
