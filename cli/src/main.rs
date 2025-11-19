use clap::{Parser, Subcommand};
use serde_json::Value;
use std::{error::Error, fs::File, io::BufReader};

#[derive(Parser, Debug)]
struct JSONCodeGen {
    /// json filepath
    #[arg(short, long)]
    filepath: String,

    #[command(subcommand)]
    lang: Lang,
}

#[derive(Subcommand, Debug)]
enum Lang {
    Java,
    Rust,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = JSONCodeGen::parse();

    let file = File::open(args.filepath)?;
    let reader = BufReader::new(file);

    let json: Value = serde_json::from_reader(reader)?;
    let mut stdout = std::io::stdout().lock();

    match args.lang {
        Lang::Java => jsoncodegen_java::codegen(json, &mut stdout)?,
        Lang::Rust => jsoncodegen_rust::codegen(json, &mut stdout)?,
    }

    Ok(())
}
