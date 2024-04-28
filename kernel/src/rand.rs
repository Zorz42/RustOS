pub struct Rng {
    curr: u64,
}

const fn advance(val: u64) -> u64 {
    return val.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
}

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Rng { curr: seed }
    }

    pub fn get(&mut self, from: u64, to: u64) -> u64 {
        debug_assert!(from < to);
        self.curr = advance(self.curr);
        self.curr % (to - from) + from
    }
}
