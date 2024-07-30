use std::{fs::File, io::BufReader};

use clap::{Parser, Subcommand};
use codegen::JavaOpts;
use serde_json::Value;

mod codegen;
mod schema_extraction;

#[derive(Parser, Debug)]
struct JSONCodeGen {
    /// json filepath
    #[arg(short, long)]
    filepath: String,

    #[command(subcommand)]
    lang: LangOpts,
}

#[derive(Subcommand, Debug)]
enum LangOpts {
    Java(JavaOpts),
}

fn main() -> anyhow::Result<()> {
    let args = JSONCodeGen::parse();

    let file = File::open(args.filepath)?;
    let reader = BufReader::new(file);

    let json: Value = serde_json::from_reader(reader)?;
    let schema = schema_extraction::process(json);
    let mut stdout = std::io::stdout().lock();

    match args.lang {
        LangOpts::Java(opts) => codegen::java(&schema, opts, &mut stdout)?,
    }

    Ok(())
}
