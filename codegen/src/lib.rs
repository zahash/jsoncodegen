use std::io;

use jsoncodegen_sink::Sink;

pub trait CodeGen {
    fn codegen(&mut self, json: serde_json::Value, sink: &mut dyn Sink) -> io::Result<()>;
}
