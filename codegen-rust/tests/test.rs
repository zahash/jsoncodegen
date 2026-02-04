use jsoncodegen_rust::codegen;
use jsoncodegen_test_macro::fixture;
use jsoncodegen_test_utils::{Docker, Template, TestConfig};

use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

static TEST_CONFIG: LazyLock<TestConfig> = LazyLock::new(|| {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    TestConfig {
        template: Template {
            dir: crate_root.join("tests").join("template"),
            codegen_output: PathBuf::from("src").join("generated.rs"),
        },
        docker: Docker {
            image: "docker.io/library/rust:slim".into(),
            mounts: vec![],
            script: crate_root.join("tests").join("script.sh"),
        },
    }
});

#[fixture("../test-data/**/*.json")]
async fn rust_test<P: AsRef<Path>>(input_filepath: P) {
    jsoncodegen_test_utils::test(&TEST_CONFIG, codegen, &input_filepath).await;
}
