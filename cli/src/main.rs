use clap::Parser;
use std::{env, error::Error, fs::File, path::PathBuf};
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

    // TODO: download the wasm binary if not available locally
    let codegen_wasm_path = runtime_dir.join(format!("jsoncodegen-{}.wasm", args.lang));
    if !codegen_wasm_path.is_file() {
        return Err("unsupported language".into());
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
