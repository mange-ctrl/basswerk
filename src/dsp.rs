//! Enkla DSP-byggstenar: oscillatorer, brus, enveloper och filter.

/// Deterministisk slumptalare (xorshift64).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// Brus i intervallet -1.0 .. 1.0
    pub fn noise(&mut self) -> f32 {
        let v = (self.next_u64() >> 11) as f32 / (1u64 << 53) as f32;
        v * 2.0 - 1.0
    }
}

/// Sinus ur en fas i varv (0.0 .. 1.0).
pub fn sine(phase: f32) -> f32 {
    (phase * std::f32::consts::TAU).sin()
}

/// Mjuk sågtand ur en fas i varv.
pub fn saw(phase: f32) -> f32 {
    2.0 * (phase - phase.floor()) - 1.0
}

/// Exponentiell avklingning: 1.0 vid t=0, ~0 vid t=len.
pub fn decay(t: f32, len: f32) -> f32 {
    if t < 0.0 || t > len || len <= 0.0 {
        return 0.0;
    }
    (-5.0 * (t / len)).exp()
}

/// Linjär attack följd av exponentiellt fall (AR-envelope).
pub fn ar(t: f32, attack: f32, len: f32) -> f32 {
    if t < 0.0 || t > len {
        return 0.0;
    }
    if t < attack && attack > 0.0 {
        t / attack
    } else {
        decay(t - attack, len - attack)
    }
}

/// Enkelt enpoligt lågpassfilter.
pub struct LowPass {
    y: f32,
    a: f32,
}

impl LowPass {
    /// cutoff i Hz vid given samplerate.
    pub fn new(cutoff_hz: f32, sr: f32) -> LowPass {
        let x = (-std::f32::consts::TAU * cutoff_hz / sr).exp();
        LowPass { y: 0.0, a: 1.0 - x }
    }
    pub fn process(&mut self, x: f32) -> f32 {
        self.y += self.a * (x - self.y);
        self.y
    }
}
