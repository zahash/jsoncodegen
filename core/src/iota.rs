pub struct Iota {
    n: usize,
}

impl Iota {
    pub fn new() -> Self {
        Self { n: 0 }
    }

    pub fn get(&mut self) -> usize {
        let n = self.n;
        self.n += 1;
        n
    }
}
