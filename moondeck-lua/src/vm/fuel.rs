pub struct Fuel {
    remaining: i32,
}

impl Fuel {
    pub fn with(amount: i32) -> Self {
        Self { remaining: amount }
    }

    /// Consume `n` units of fuel. Returns `true` if fuel is exhausted.
    pub fn consume(&mut self, n: i32) -> bool {
        self.remaining -= n;
        self.remaining <= 0
    }

    pub fn remaining(&self) -> i32 {
        self.remaining
    }
}
