use jsoncodegen_test_macro::generate_tests;

generate_tests!(
    "../test-data",
    language: "rust",
    template_dir: "tests/template",
    codegen_fn: |json, out| jsoncodegen_rust::codegen(json, out),
    docker_image: "docker.io/library/rust:slim",
    docker_command: "set -e; /usr/local/cargo/bin/cargo run --quiet < /data/input.json > /data/output.json;",
    extra_volumes: vec![],
    work_dir: "/workspace",
    source_path: "src/generated.rs",
);
