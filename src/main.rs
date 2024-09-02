use std::{fs::File, io::BufReader};

use clap::{Parser, Subcommand};
use serde_json::Value;

mod codegen;
mod schema_extraction;

use schema_extraction::extract;
// use codegen::JavaOpts;

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
    // Java(JavaOpts),
}

fn main() -> anyhow::Result<()> {
    // let args = JSONCodeGen::parse();

    // let file = File::open(args.filepath)?;
    // let reader = BufReader::new(file);

    // let json: Value = serde_json::from_reader(reader)?;
    // let schema = schema_extraction::extract(json);
    // let mut stdout = std::io::stdout().lock();

    // match args.lang {
    //     LangOpts::Java(opts) => codegen::java(schema, opts, &mut stdout)?,
    // }

    let json = serde_json::from_str(
        r#"
                {
                    "h": [
                        "mixed", true, 
                        ["nested", "arr"], ["arr2"], [123], [true, 27, [22.34]], 
                        {"k1": "v1", "k3": true}, {"k1": 23, "k3": false}, {"k2": "v2", "k3": true}
                    ]
                }
                "#,
    )
    .unwrap();

    let schema = extract(json);

    println!("{:#?}", schema);

    Ok(())
}
