use glob::glob;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::env;
use std::path::PathBuf;
use syn::{ItemFn, LitStr, Token, parse_macro_input, punctuated::Punctuated};

struct FixtureArgs {
    globs: Punctuated<LitStr, Token![,]>,
}

impl syn::parse::Parse for FixtureArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(FixtureArgs {
            globs: Punctuated::parse_terminated(input)?,
        })
    }
}

#[proc_macro_attribute]
pub fn fixture(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as FixtureArgs);
    let input_fn = parse_macro_input!(input as ItemFn);
    let fn_name = &input_fn.sig.ident;

    let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut tests = Vec::new();

    for glob_lit in &args.globs {
        let absolute_glob_path = base_path.join(glob_lit.value());
        let absolute_glob_str = absolute_glob_path.to_str().expect(&format!(
            "non-utf8 absolute glob path :: {}",
            absolute_glob_path.display()
        ));

        let paths = glob(absolute_glob_str).expect("Failed to read glob pattern");
        for entry in paths {
            let path = entry.expect("Failed to read glob entry");
            if path.is_file() {
                // Generate a unique test name
                // e.g. tests/data/user.json -> tests_data_user_json
                let safe_name = path
                    .strip_prefix(&base_path)
                    .unwrap_or_else(|_| &path)
                    .to_string_lossy()
                    .replace(|c: char| !c.is_alphanumeric(), "_")
                    .to_lowercase();
                let test_name = format_ident!("{}", safe_name);
                let path_str = path
                    .to_str()
                    .expect(&format!("non-utf8 glob path :: {}", path.display()));

                tests.push(quote! {
                    #[tokio::test]
                    async fn #test_name() {
                        // Recompile if this specific file changes
                        const _: &[u8] = include_bytes!(#path_str);

                        let path = std::path::PathBuf::from(#path_str);
                        #fn_name(path).await;
                    }
                });
            }
        }
    }

    if tests.is_empty() {
        return syn::Error::new_spanned(
            &args.globs,
            "No files found matching the provided glob patterns",
        )
        .to_compile_error()
        .into();
    }

    TokenStream::from(quote! {
        #(#tests)*
        #input_fn
    })
}
