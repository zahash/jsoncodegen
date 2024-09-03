mod codegen;
mod schema_extraction;

use clap::{Parser, Subcommand};
use schema_extraction::extract;
use serde_json::Value;
use std::{fs::File, io::BufReader};
// use codegen::JavaOpts;

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
    // Java(JavaOpts),
    Rust,
}

fn main() -> anyhow::Result<()> {
    let args = JSONCodeGen::parse();

    let file = File::open(args.filepath)?;
    let reader = BufReader::new(file);

    let json: Value = serde_json::from_reader(reader)?;
    let schema = extract(json);
    let mut stdout = std::io::stdout().lock();

    match args.lang {
        // LangOpts::Java(opts) => codegen::java(schema, opts, &mut stdout)?,
        Lang::Rust => codegen::rust(schema, &mut stdout)?,
    }

    Ok(())
}
