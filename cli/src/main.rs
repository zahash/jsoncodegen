use clap::{Parser, Subcommand};
use jsoncodegen_utils::default_runtime_dir;
use std::{
    error::Error,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;

#[derive(Parser, Debug)]
#[command(about = "JSON code generator")]
struct Args {
    #[arg(long, env("JSONCODEGEN_RUNTIME"), default_value_os_t = default_runtime_dir())]
    runtime_dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate code from a JSON file
    Gen(GenArgs),

    /// Update generated artifacts
    #[command(subcommand)]
    Update(UpdateArgs),
}

#[derive(Parser, Debug)]
struct GenArgs {
    /// input json filepath
    #[arg(short, long)]
    filepath: PathBuf,

    /// codegen language
    #[arg(long)]
    lang: String,

    /// Optional output file; if omitted, prints to stdout
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum UpdateArgs {
    /// Update a specific language
    Lang {
        /// Language to update
        lang: String,
    },

    /// Update all languages
    All,
}

fn main() -> Result<(), Box<dyn Error>> {
    let Args {
        runtime_dir,
        command,
    } = Args::parse();

    match command {
        Commands::Gen(gen_args) => {
            let codegen_wasm_path = runtime_dir.join(lang_2_wasm_filename(&gen_args.lang));

            // Check if WASM binary exists locally, if not download it
            if !codegen_wasm_path.is_file() {
                eprintln!("WASM binary not found locally");
                fetch_latest_wasm_release(&gen_args.lang, &codegen_wasm_path)?;
            }

            let ctx = {
                let mut builder = WasiCtxBuilder::new();

                builder
                    .stdin({
                        let file = File::open(gen_args.filepath)?;
                        wasmtime_wasi::cli::InputFile::new(file)
                    })
                    .stderr(wasmtime_wasi::cli::stderr());

                match gen_args.output {
                    Some(out_path) => {
                        let file = File::create(out_path)?;
                        builder.stdout(wasmtime_wasi::cli::OutputFile::new(file))
                    }
                    None => builder.stdout(wasmtime_wasi::cli::stdout()),
                };

                builder.build_p1()
            };

            let engine = Engine::default();
            let mut linker = Linker::<wasmtime_wasi::p1::WasiP1Ctx>::new(&engine);
            wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |s| s)?;

            let module = Module::from_file(&engine, codegen_wasm_path)?;
            let mut store = Store::new(&engine, ctx);
            let instance = linker.instantiate(&mut store, &module)?;

            let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
            start.call(&mut store, ())?;
        }
        Commands::Update(update_args) => match update_args {
            UpdateArgs::Lang { lang } => {
                fetch_latest_wasm_release(&lang, &runtime_dir.join(lang_2_wasm_filename(&lang)))?
            }
            UpdateArgs::All => unimplemented!(),
        },
    }

    Ok(())
}

fn fetch_latest_wasm_release(lang: &str, dest_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    eprintln!("Fetching latest WASM release info for language `{}`", lang);

    let client = reqwest::blocking::Client::builder()
        .user_agent("jsoncodegen")
        .build()?;

    let url = format!("https://zahash.github.io/{}", lang_2_wasm_filename(lang));

    let response = client.get(&url).send()?;
    if !response.status().is_success() {
        return Err(format!("Failed to download: HTTP {}", response.status()).into());
    }
    let bytes = response.bytes()?;

    // Ensure the parent directory exists
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(dest_path)?;
    file.write_all(&bytes)?;

    eprintln!("Successfully downloaded to: {}", dest_path.display());
    Ok(())
}

fn lang_2_wasm_filename(lang: &str) -> String {
    format!("jsoncodegen-{}-wasm32-wasip1.wasm", lang)
}
