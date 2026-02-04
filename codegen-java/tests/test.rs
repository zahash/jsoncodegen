use jsoncodegen_java::codegen;
use jsoncodegen_test_macro::fixture;
use jsoncodegen_test_utils::{Docker, Manifest, Mount, Template};

use std::{
    env,
    path::{Path, PathBuf},
    sync::LazyLock,
};

static MANIFEST: LazyLock<Manifest> = LazyLock::new(|| {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    Manifest {
        template: Template {
            dir: crate_root.join("tests").join("template"),
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
            script: crate_root.join("tests").join("script.sh"),
        },
    }
});

#[fixture("../test-data/*.json")]
async fn java_test<P: AsRef<Path>>(input_filepath: P) {
    jsoncodegen_test_utils::test(&MANIFEST, codegen, &input_filepath).await;
}
