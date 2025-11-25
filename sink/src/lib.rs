use std::io;

pub trait Sink {
    fn sink<'sink>(&'sink mut self, name: &str) -> io::Result<&'sink mut dyn io::Write>;
}
