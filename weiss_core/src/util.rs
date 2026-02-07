/// Deterministic xorshift64* RNG for reproducibility.
#[derive(Clone, Copy, Debug, Hash)]
pub struct Rng64 {
    state: u64,
}

impl Rng64 {
    /// Create a new RNG (zero seed is remapped).
    pub fn new(seed: u64) -> Self {
        let mut s = seed;
        if s == 0 {
            s = 0x9E3779B97F4A7C15;
        }
        Self { state: s }
    }

    /// Generate the next u64.
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Generate the next u32.
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Read the current internal state.
    pub fn state(&self) -> u64 {
        self.state
    }

    /// Generate a boolean with 50/50 probability.
    pub fn next_bool(&mut self) -> bool {
        (self.next_u64() & 1) == 1
    }

    /// Generate a uniform integer in [0, upper).
    pub fn gen_range(&mut self, upper: usize) -> usize {
        if upper == 0 {
            return 0;
        }
        (self.next_u64() as usize) % upper
    }

    /// Shuffle a slice in-place.
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.gen_range(i + 1);
            slice.swap(i, j);
        }
    }
}
