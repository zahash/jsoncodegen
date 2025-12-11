use clap::Parser;
use serde::Deserialize;
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

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

fn fetch_latest_wasm_release(lang: &str, dest_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    eprintln!("Fetching latest WASM release info for language `{}`", lang);

    let client = reqwest::blocking::Client::builder()
        .user_agent("jsoncodegen")
        .build()?;
    let mut releases: Vec<Release> = client
        .get("https://api.github.com/repos/zahash/jsoncodegen/releases")
        .send()?
        .json()?;

    // Filter releases that match the pattern: jsoncodegen-{lang}-wasm32-wasip1-{version}
    let tag_prefix = format!("jsoncodegen-{}-wasm32-wasip1-", lang);
    releases.retain(|release| release.tag_name.starts_with(&tag_prefix));

    // Sort by version number (descending) - extract the number after the last dash
    let latest_release = releases
        .into_iter()
        .max_by_key(|release| {
            release
                .tag_name
                .strip_prefix(&tag_prefix)
                .and_then(|version| version.parse::<usize>().ok())
                .unwrap_or(0)
        })
        .ok_or_else(|| format!("No WASM releases found for language `{}`", lang))?;

    eprintln!("latest release found: {}", latest_release.tag_name);

    let asset_name = format!("jsoncodegen-{}-wasm32-wasip1.wasm", lang);
    let asset = latest_release
        .assets
        .into_iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            format!(
                "WASM asset `{}` not found in release `{}`",
                asset_name, latest_release.tag_name
            )
        })?;

    eprintln!(
        "Downloading WASM binary from: {}",
        asset.browser_download_url
    );

    let response = client.get(asset.browser_download_url).send()?;
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
