use std::io::{self, Stdin, stdout};

use jsoncodegen_rust::codegen;

fn main() -> io::Result<()> {
    let json = serde_json::from_reader::<Stdin, serde_json::Value>(io::stdin())?;
    codegen(json, &mut stdout())
}
