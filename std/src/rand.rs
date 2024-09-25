pub struct Rng {
    curr: u64,
}

const fn advance(val: u64) -> u64 {
    val.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407)//.div_ceil(4).wrapping_mul(5)
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
