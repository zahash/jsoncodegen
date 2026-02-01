use jsoncodegen_test_macro::generate_tests;
use std::{
    env,
    path::PathBuf,
    sync::LazyLock,
};

static M2: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = env::home_dir()
        .expect("unable to determine home_dir")
        .join(".m2");
    std::fs::create_dir_all(&path).expect("Failed to create .m2 directory");
    path
});

generate_tests!(
    "../test-data",
    language: "java",
    template_dir: "tests/template",
    codegen_fn: |json, out| jsoncodegen_java::codegen(json, out),
    docker_image: "docker.io/library/maven:3.9.9-eclipse-temurin-17",
    docker_command: "set -e; mvn clean package; mvn dependency:copy-dependencies -DoutputDirectory=target/lib -DincludeScope=runtime; java -jar target/jsoncodegen.jar < /data/input.json > /data/output.json;",
    extra_volumes: vec![(M2.clone(), "/root/.m2", "")],
    work_dir: "/workspace",
    source_path: "src/JsonCodeGen.java",
);
