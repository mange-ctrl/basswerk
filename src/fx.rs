//! Effekter: algoritmiskt reverb (Schroeder) och stereodelay.

struct Comb {
    buf: Vec<f32>,
    idx: usize,
    fb: f32,
    damp: f32,
    store: f32,
}

impl Comb {
    fn new(len: usize, fb: f32, damp: f32) -> Comb {
        Comb {
            buf: vec![0.0; len.max(1)],
            idx: 0,
            fb,
            damp,
            store: 0.0,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let y = self.buf[self.idx];
        self.store = y * (1.0 - self.damp) + self.store * self.damp;
        self.buf[self.idx] = x + self.store * self.fb;
        self.idx = (self.idx + 1) % self.buf.len();
        y
    }
}

struct Allpass {
    buf: Vec<f32>,
    idx: usize,
    g: f32,
}

impl Allpass {
    fn new(len: usize, g: f32) -> Allpass {
        Allpass {
            buf: vec![0.0; len.max(1)],
            idx: 0,
            g,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let y = self.buf[self.idx];
        let out = -x + y;
        self.buf[self.idx] = x + y * self.g;
        self.idx = (self.idx + 1) % self.buf.len();
        out
    }
}

/// Ett enkelt Schroeder-reverb. `size` skalar rummet (1.0 = normalt).
pub struct Reverb {
    combs: Vec<Comb>,
    aps: Vec<Allpass>,
}

impl Reverb {
    pub fn new(sr: f32, size: f32) -> Reverb {
        let ms = |m: f32| ((m * size / 1000.0) * sr) as usize;
        Reverb {
            combs: vec![
                Comb::new(ms(29.7), 0.805, 0.35),
                Comb::new(ms(37.1), 0.827, 0.35),
                Comb::new(ms(41.1), 0.783, 0.35),
                Comb::new(ms(43.7), 0.764, 0.35),
            ],
            aps: vec![Allpass::new(ms(5.0), 0.7), Allpass::new(ms(1.7), 0.7)],
        }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let mut sum = 0.0;
        for c in &mut self.combs {
            sum += c.process(x);
        }
        let mut y = sum / self.combs.len() as f32;
        for a in &mut self.aps {
            y = a.process(y);
        }
        y
    }
}

/// Stereodelay med korsvis återkoppling.
pub struct Delay {
    l: Vec<f32>,
    r: Vec<f32>,
    i: usize,
    fb: f32,
}

impl Delay {
    pub fn new(sr: f32, time_sec: f32, fb: f32) -> Delay {
        let n = ((time_sec * sr) as usize).max(1);
        Delay {
            l: vec![0.0; n],
            r: vec![0.0; n],
            i: 0,
            fb,
        }
    }

    /// Tar en monosample, ger tillbaka vått i (vänster, höger).
    pub fn process(&mut self, x: f32) -> (f32, f32) {
        let yl = self.l[self.i];
        let yr = self.r[self.i];
        self.l[self.i] = x + yr * self.fb;
        self.r[self.i] = x + yl * self.fb;
        self.i = (self.i + 1) % self.l.len();
        (yl, yr)
    }
}
