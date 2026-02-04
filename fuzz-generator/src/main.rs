use jsoncodegen_test_utils::{Docker, Manifest, Mount, Template};
use rand::distributions::Alphanumeric;
use rand::prelude::*;
use serde_json::{json, Map, Value};
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task;

const MAX_DEPTH: usize = 8;
const ITERATIONS: usize = 500;

#[tokio::main]
async fn main() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    // Ensure test-data output dir exists
    let output_dir = workspace_root.join("test-data").join("fuzz-failures");
    fs::create_dir_all(&output_dir).unwrap();

    // Prepare Manifests
    let rust_manifest = Arc::new(Manifest {
        template: Template {
            dir: workspace_root.join("codegen-rust").join("tests").join("template"),
            codegen_output: PathBuf::from("src").join("generated.rs"),
        },
        docker: Docker {
            image: "docker.io/library/rust:slim".into(),
            mounts: vec![],
            script: workspace_root.join("codegen-rust").join("tests").join("script.sh"),
        },
    });

    let java_manifest = Arc::new(Manifest {
        template: Template {
            dir: workspace_root.join("codegen-java").join("tests").join("template"),
            codegen_output: PathBuf::from("src").join("JsonCodeGen.java"),
        },
        docker: Docker {
            image: "docker.io/library/maven:3.9.9-eclipse-temurin-17".into(),
            mounts: env::home_dir()
                .map(|home_dir| {
                    vec![Mount {
                        source: home_dir.join(".m2"),
                        target: PathBuf::from("/root").join(".m2"),
                    }]
                })
                .unwrap_or_default(),
            script: workspace_root.join("codegen-java").join("tests").join("script.sh"),
        },
    });

    println!("Starting fuzzer for {} iterations...", ITERATIONS);

    let mut failures = 0;

    for i in 0..ITERATIONS {
        let input = generate_random_json(0);

        // Save to temp file
        let temp_dir = env::temp_dir().join("jsoncodegen-fuzz");
        fs::create_dir_all(&temp_dir).unwrap();
        let input_path = temp_dir.join("input.json");
        fs::write(&input_path, serde_json::to_string_pretty(&input).unwrap()).unwrap();

        let mut failed = false;
        let mut reasons = Vec::new();

        // Run Rust Test
        {
            let m = rust_manifest.clone();
            let p = input_path.clone();
            let handle = tokio::spawn(async move {
                jsoncodegen_test_utils::test(&m, jsoncodegen_rust::codegen, &p).await;
            });

            match handle.await {
                Ok(_) => {},
                Err(e) if e.is_panic() => {
                    failed = true;
                    reasons.push("rust");
                    println!("Iteration {}: Rust failure", i);
                },
                Err(e) => {
                    println!("Iteration {}: Rust task error: {:?}", i, e);
                }
            }
        }

        // Run Java Test
        {
            let m = java_manifest.clone();
            let p = input_path.clone();
            let handle = tokio::spawn(async move {
                jsoncodegen_test_utils::test(&m, jsoncodegen_java::codegen, &p).await;
            });

            match handle.await {
                Ok(_) => {},
                Err(e) if e.is_panic() => {
                    failed = true;
                    reasons.push("java");
                    println!("Iteration {}: Java failure", i);
                },
                Err(e) => {
                    println!("Iteration {}: Java task error: {:?}", i, e);
                }
            }
        }

        if failed {
            failures += 1;
            let mut hasher = DefaultHasher::new();
            input.to_string().hash(&mut hasher);
            let hash = hasher.finish();
            let name = format!("fuzz_{}_{}.json", reasons.join("_"), hash);
            let target = output_dir.join(&name);
            match fs::copy(&input_path, &target) {
                Ok(_) => println!("Saved failure to {}", name),
                Err(e) => println!("Failed to save failure {}: {:?}", name, e),
            }
        }
    }

    println!("Fuzzing complete. Found {} failures.", failures);
}

fn generate_random_json(depth: usize) -> Value {
    let mut rng = thread_rng();

    if depth >= MAX_DEPTH {
        // Return primitive
        return generate_primitive(&mut rng);
    }

    // 20% primitive, 40% object, 40% array
    let choice = rng.gen_range(0..100);
    if choice < 20 {
        generate_primitive(&mut rng)
    } else if choice < 60 {
        // Object
        let mut map = Map::new();
        let num_fields = rng.gen_range(0..10);
        for _ in 0..num_fields {
            let key = generate_key(&mut rng);
            map.insert(key, generate_random_json(depth + 1));
        }
        Value::Object(map)
    } else {
        // Array
        let num_items = rng.gen_range(0..10);
        let mut vec = Vec::new();
        for _ in 0..num_items {
            vec.push(generate_random_json(depth + 1));
        }
        Value::Array(vec)
    }
}

fn generate_primitive(rng: &mut ThreadRng) -> Value {
    match rng.gen_range(0..5) {
        0 => Value::Null,
        1 => Value::Bool(rng.gen()),
        2 => {
            // Numbers: Interger or Float
            if rng.gen_bool(0.5) {
                Value::Number(serde_json::Number::from(rng.gen::<i64>()))
            } else {
                let f: f64 = rng.gen();
                 // serde_json::Number::from_f64 returns Option
                Value::Number(serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)))
            }
        },
        3 => Value::String(generate_string(rng)),
        4 => {
             // Edge case numbers
             let candidates = vec![0, 1, -1, i64::MAX, i64::MIN];
             Value::Number(serde_json::Number::from(*candidates.choose(rng).unwrap()))
        }
        _ => Value::Null,
    }
}

fn generate_key(rng: &mut ThreadRng) -> String {
    // Mix of simple, empty, special chars, keywords
    match rng.gen_range(0..10) {
        0 => "".to_string(), // Empty key
        1 => "class".to_string(), // Java Keyword
        2 => "type".to_string(), // Rust/Common Keyword
        3 => "null".to_string(),
        4 => "true".to_string(),
        5 => {
            // Special chars
            let special = vec!["-", " ", "@", "$", ".", "/", "\\", "\"", "\n"];
            format!("key{}", special.choose(rng).unwrap())
        },
        _ => generate_string(rng),
    }
}

fn generate_string(rng: &mut ThreadRng) -> String {
    if rng.gen_bool(0.1) {
        return "".to_string();
    }
    let len = rng.gen_range(1..20);
    rng.sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
