use proc_macro::TokenStream;
use quote::quote;
use std::env;
use std::fs;
use std::path::Path;
use syn::{Error, LitStr, Result, parse_macro_input};

#[proc_macro]
pub fn generate_tests(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a String Literal
    // If this fails, `parse_macro_input!` automatically emits a compile error and returns.
    let input_dir = parse_macro_input!(input as LitStr);

    // Call the implementation and handle any Results
    match generate_tests_impl(input_dir) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn generate_tests_impl(dir_literal: LitStr) -> Result<proc_macro2::TokenStream> {
    let dir_str = dir_literal.value();

    // 1. Resolve the path relative to the crate root safely
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        Error::new(
            dir_literal.span(),
            "Failed to read CARGO_MANIFEST_DIR env variable",
        )
    })?;

    let test_data_path = Path::new(&manifest_dir)
        .parent()
        .ok_or_else(|| {
            Error::new(
                dir_literal.span(),
                "Failed to get parent directory of CARGO_MANIFEST_DIR",
            )
        })?
        .join(&dir_str);

    // 2. Validate directory existence
    if !test_data_path.exists() {
        return Err(Error::new(
            dir_literal.span(),
            format!(
                "Test data directory does not exist: {}",
                test_data_path.display()
            ),
        ));
    }

    let mut test_fns = Vec::new();

    // 3. Read directory safely
    let entries = fs::read_dir(&test_data_path).map_err(|e| {
        Error::new(
            dir_literal.span(),
            format!("Failed to read directory: {}", e),
        )
    })?;

    for entry in entries {
        // Handle read errors for individual entries
        let entry = entry
            .map_err(|e| Error::new(dir_literal.span(), format!("Failed to read entry: {}", e)))?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
            // Safe filename extraction
            let stem = path
                .file_stem()
                .ok_or_else(|| Error::new(dir_literal.span(), "Failed to extract file stem"))?
                .to_str()
                .ok_or_else(|| Error::new(dir_literal.span(), "Filename contains invalid UTF-8"))?;

            // Create a valid Rust identifier (e.g., "user-data" -> "test_user_data")
            let safe_name = stem.replace(|c: char| !c.is_alphanumeric(), "_");
            let fn_name = syn::Ident::new(
                &format!("test_{}", safe_name),
                proc_macro2::Span::call_site(),
            );

            // Safe path conversion for the generated string
            let path_str = path
                .to_str()
                .ok_or_else(|| Error::new(dir_literal.span(), "Path contains invalid UTF-8"))?;

            // 4. Generate the test function
            test_fns.push(quote! {
                #[tokio::test]
                async fn #fn_name() {
                    run_test(std::path::Path::new(#path_str)).await;
                }
            });
        }
    }

    if test_fns.is_empty() {
        // Optional warning if no JSON files were found
        return Err(Error::new(
            dir_literal.span(),
            "No JSON files found in the specified directory",
        ));
    }

    Ok(quote! {
        #(#test_fns)*
    })
}
