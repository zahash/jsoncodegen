use clap::Parser;
use std::{
    env,
    error::Error,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;

#[derive(Parser, Debug)]
struct JSONCodeGen {
    /// input json filepath
    #[arg(short, long)]
    filepath: String,

    /// codegen language
    #[arg(long)]
    lang: String,

    /// Optional output file; if omitted, prints to stdout
    #[arg(short, long)]
    output: Option<PathBuf>,

    #[arg(long, env("JSONCODEGEN_RUNTIME"))]
    runtime_dir: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = JSONCodeGen::parse();

    let runtime_dir = args
        .runtime_dir
        .or_else(|| {
            env::home_dir().map(|mut home| {
                home.push(".jsoncodegen");
                home
            })
        })
        .ok_or("default runtime directory unavailable. please specify an alternate manually.")?;

    let codegen_wasm_path =
        runtime_dir.join(format!("jsoncodegen-{}-wasm32-wasip1.wasm", args.lang));

    // Check if WASM binary exists locally, if not download it
    if !codegen_wasm_path.is_file() {
        eprintln!("WASM binary not found locally");
        fetch_latest_wasm_release(&args.lang, &codegen_wasm_path)?;
    }

    let ctx = {
        let mut builder = WasiCtxBuilder::new();

        builder
            .stdin({
                let file = File::open(args.filepath)?;
                wasmtime_wasi::cli::InputFile::new(file)
            })
            .stderr(wasmtime_wasi::cli::stderr());

        match args.output {
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

    Ok(())
}

fn fetch_latest_wasm_release(lang: &str, dest_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    eprintln!("Fetching latest WASM release info for language `{}`", lang);

    let client = reqwest::blocking::Client::builder()
        .user_agent("jsoncodegen")
        .build()?;

    let url = format!(
        "https://zahash.github.io/jsoncodegen-{}-wasm32-wasip1.wasm",
        lang
    );

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
