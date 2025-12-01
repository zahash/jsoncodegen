use clap::Parser;
use jsoncodegen_dispatch::dispatch;
use serde_json::Value;
use std::{
    error::Error,
    fs::File,
    io::{BufReader, Write, stdout},
    path::PathBuf,
};

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
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = JSONCodeGen::parse();

    let file = File::open(args.filepath)?;
    let reader = BufReader::new(file);

    let json: Value = serde_json::from_reader(reader)?;

    let mut out: Box<dyn Write> = match args.output {
        Some(output_filepath) => Box::new(File::create(output_filepath)?),
        None => Box::new(stdout().lock()),
    };

    match dispatch(&args.lang, json, &mut out)? {
        true => Ok(()),
        false => Err(format!("`{}` language not supported", args.lang).into()),
    }
}
