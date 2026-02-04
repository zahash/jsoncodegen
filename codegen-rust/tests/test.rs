use jsoncodegen_rust::codegen;
use jsoncodegen_test_macro::fixture;
use jsoncodegen_test_utils::{Docker, Manifest, Template};

use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

static MANIFEST: LazyLock<Manifest> = LazyLock::new(|| {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    Manifest {
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

#[fixture("../test-data/*.json")]
async fn rust_test<P: AsRef<Path>>(input_filepath: P) {
    jsoncodegen_test_utils::test(&MANIFEST, codegen, &input_filepath).await;
}
