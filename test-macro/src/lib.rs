use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use std::{fs, path::PathBuf, env};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, LitStr, Token,
};

struct TestConfigInput {
    directory: LitStr,
    _comma: Token![,],
    fields: Punctuated<ConfigField, Token![,]>,
}

struct ConfigField {
    key: Ident,
    _colon: Token![:],
    value: Expr,
}

impl Parse for TestConfigInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(TestConfigInput {
            directory: input.parse()?,
            _comma: input.parse()?,
            fields: Punctuated::parse_terminated(input)?,
        })
    }
}

impl Parse for ConfigField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ConfigField {
            key: input.parse()?,
            _colon: input.parse()?,
            value: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn generate_tests(input: TokenStream) -> TokenStream {
    let TestConfigInput {
        directory,
        fields,
        ..
    } = syn::parse_macro_input!(input as TestConfigInput);

    let mut language = None;
    let mut template_dir = None;
    let mut codegen_fn = None;
    let mut docker_image = None;
    let mut docker_command = None;
    let mut extra_volumes = quote! { vec![] };
    let mut work_dir = None;
    let mut source_path = None;

    for field in fields {
        let key = field.key.to_string();
        let value = field.value;
        match key.as_str() {
            "language" => language = Some(value),
            "template_dir" => template_dir = Some(value),
            "codegen_fn" => codegen_fn = Some(value),
            "docker_image" => docker_image = Some(value),
            "docker_command" => docker_command = Some(value),
            "extra_volumes" => extra_volumes = quote! { #value },
            "work_dir" => work_dir = Some(value),
            "source_path" => source_path = Some(value),
            _ => panic!("Unknown config key: {}", key),
        }
    }

    let language = language.expect("Missing 'language'");
    let template_dir = template_dir.expect("Missing 'template_dir'");
    let codegen_fn = codegen_fn.expect("Missing 'codegen_fn'");
    let docker_image = docker_image.expect("Missing 'docker_image'");
    let docker_command = docker_command.expect("Missing 'docker_command'");
    let work_dir = work_dir.expect("Missing 'work_dir'");
    let source_path = source_path.expect("Missing 'source_path'");

    let dir_str = directory.value();

    // Try to find the directory.
    // If the path starts with `../`, we might be in the package dir.
    // If it doesn't, we might be in the workspace root.
    // We try the path as given, and if it fails, we try some alternatives relative to CWD.
    let path = PathBuf::from(&dir_str);

    // Helper to find valid path
    let resolved_path = if path.exists() {
        Some(path)
    } else if let Ok(cwd) = env::current_dir() {
        // Try joining with CWD (redundant if path is relative, but useful for debugging)
        let p1 = cwd.join(&dir_str);
        if p1.exists() {
             Some(p1)
        } else {
            // Try explicit workspace root heuristic?
            // If we are in `codegen-java` and path is `../test-data`, it should have worked.
            // If we are in workspace root, `../test-data` fails, but `test-data` works.
            // Let's try to be smart: if input is `../test-data`, try `test-data`.
            if dir_str.starts_with("../") {
                let stripped = dir_str.trim_start_matches("../");
                let p2 = PathBuf::from(stripped);
                if p2.exists() {
                    Some(p2)
                } else {
                    None
                }
            } else {
                None
            }
        }
    } else {
        None
    };

    let path = resolved_path.unwrap_or_else(|| {
        panic!(
            "Failed to find test data directory: '{}'. Current working dir: {:?}",
            dir_str,
            env::current_dir().unwrap_or_default()
        )
    });

    let entries = fs::read_dir(&path).expect(&format!("Failed to read directory: {:?}", path));

    // We need to pass the RELATIVE path from the crate root (CARGO_MANIFEST_DIR) to the test function.
    // The `path` variable here is what we found at compile time.
    // The runtime code uses `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(...)`.
    // We need to ensure the joined path is correct.
    // If we resolved `../test-data` to `test-data` (because we are in root),
    // but the runtime code (test) runs in `codegen-java`, it expects `../test-data`.
    // The `directory` argument passed to the macro (`"../test-data"`) is what the user INTENDED relative to the crate.
    // So we should use `dir_str` in the generated code, assuming the user knows the runtime relative path.
    // The fact that we found it at a different location at compile time (due to CWD differences) shouldn't change the runtime path if `cargo test` runs in the crate dir.

    let tests = entries.map(|entry| {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "json") {
            let stem = path.file_stem().unwrap().to_string_lossy();
            let test_name = Ident::new(
                &format!("test_{}", stem.to_case(Case::Snake)),
                proc_macro2::Span::call_site(),
            );

            // We use the filename, and join it with the configured directory string.
            let filename = path.file_name().unwrap().to_string_lossy();
            let input_file_path = format!("{}/{}", dir_str, filename);

            quote! {
                #[tokio::test]
                async fn #test_name() {
                    let input_file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(#input_file_path);
                    let template_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(#template_dir);

                    jsoncodegen_test_utils::run_test(jsoncodegen_test_utils::TestConfig {
                        language: #language,
                        input_file,
                        template_dir,
                        codegen_fn: Box::new(#codegen_fn),
                        docker_image: #docker_image,
                        docker_command: #docker_command,
                        extra_volumes: #extra_volumes,
                        work_dir: #work_dir,
                        source_path: #source_path,
                    }).await;
                }
            }
        } else {
            quote! {}
        }
    });

    TokenStream::from(quote! {
        #(#tests)*
    })
}
