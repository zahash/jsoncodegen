use jsoncodegen_java::codegen;
use std::{error, fs, path::PathBuf};

// $DOCKER run --rm \
//     -v "test-harness/java:/workspace" \
//     -v "test-harness/java/.m2:/root/.m2" \
//     -w /workspace \
//     maven:3.9.9-eclipse-temurin-17 \
//     bash -lc '  set -e; \
//                 mvn clean package; \
//                 mvn dependency:copy-dependencies -DoutputDirectory=target/lib -DincludeScope=runtime'

// $DOCKER run --rm \
//     -v "test-harness/java:/workspace" \
//     -v "test-data:/data/in:ro" \
//     -v "test-results/java:/data/out" \
//     -w /workspace \
//     eclipse-temurin:17-jre \
//     bash -lc '  set -e; \
//                 for f in /data/in/*.json; \
//                     do base=$(basename "$f"); \
//                     echo ">> $base"; \
//                     java -jar target/jsoncodegen.jar < "$f" > "/data/out/$base"; \
//                 done; \
//                 echo "✅ Done"'

#[test]
fn analytics_events() -> Result<(), Box<dyn error::Error>> {
    prepare("analytics-events")?;

    Ok(())
}

fn prepare(test_name: &str) -> Result<(), Box<dyn error::Error>> {
    let input = fs::File::open(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("Failed to get parent directory of CARGO_MANIFEST_DIR")
            .join("test-data")
            .join(format!("{}.json", test_name)),
    )?;

    let mut output = fs::File::create(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("harness")
            .join("src")
            .join("JsonCodeGen.java"),
    )?;

    codegen(serde_json::from_reader(input)?, &mut output)?;
    Ok(())
}
