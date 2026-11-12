//! Perkussion och effekter: clap, hi-hat, riser och impact.

use crate::dsp::{LowPass, Rng};
use crate::wav::Track;

/// Handklapp: brusburst med kort svans.
pub fn clap(t: &mut Track, at: f64, gain: f32, rng: &mut Rng) {
    let start = t.frame_at(at);
    let len = (0.20 * t.sr) as usize;
    let mut lp = LowPass::new(2800.0, t.sr);
    for i in 0..len {
        let sec = i as f32 / t.sr;
        let flam = 1.0 + 0.6 * (sec * 55.0).sin();
        let amp = (-sec * 20.0).exp() * flam;
        let s = lp.process(rng.noise()) * amp * gain;
        t.add(start + i, s, s);
    }
}

/// Hi-hat: högpassat brus. `open` ger längre svans.
pub fn hat(t: &mut Track, at: f64, gain: f32, rng: &mut Rng, open: bool) {
    let start = t.frame_at(at);
    let decay = if open { 14.0 } else { 55.0 };
    let len = ((if open { 0.22 } else { 0.07 }) * t.sr) as usize;
    let mut prev = 0.0f32;
    for i in 0..len {
        let sec = i as f32 / t.sr;
        let n = rng.noise();
        let hp = n - prev; // enkel högpass
        prev = n;
        let amp = (-sec * decay).exp();
        let s = hp * amp * gain;
        t.add(start + i, s * 0.9, s * 1.1);
    }
}

/// Riser: brus som ökar i styrka och ljusnar.
pub fn riser(t: &mut Track, at: f64, dur: f64, gain: f32, rng: &mut Rng) {
    let start = t.frame_at(at);
    let len = (dur * t.sr as f64) as usize;
    for i in 0..len {
        let p = i as f32 / len as f32;
        let amp = p * p * gain;
        let tone = 0.5 + 0.5 * (p * 40.0).sin();
        let s = rng.noise() * amp * tone;
        let pan = 0.3 + 0.7 * p;
        t.add(start + i, s * (1.0 - pan * 0.3), s * pan);
    }
}

/// Virvel: brus plus en dov kropp.
pub fn snare(t: &mut Track, at: f64, gain: f32, rng: &mut Rng) {
    let start = t.frame_at(at);
    let len = (0.14 * t.sr) as usize;
    let mut prev = 0.0f32;
    let mut phase = 0.0f32;
    for i in 0..len {
        let sec = i as f32 / t.sr;
        let n = rng.noise();
        let hp = n - prev;
        prev = n;
        phase += 190.0 / t.sr;
        if phase > 1.0 {
            phase -= 1.0;
        }
        let amp = (-sec * 26.0).exp();
        let s = (hp * 0.8 + crate::dsp::sine(phase) * 0.5) * amp * gain;
        t.add(start + i, s, s);
    }
}

/// Roll: virvel som accelererar — fyller sista takten före ett drop.
pub fn fill(t: &mut Track, at: f64, dur: f64, gain: f32, rng: &mut Rng) {
    let n = 12;
    for k in 0..n {
        let p = (k as f64 + 1.0) / n as f64;
        let when = at + dur * p.powf(1.7);
        let amp = gain * (0.45 + 0.55 * p as f32);
        snare(t, when, amp, rng);
    }
}

/// Impact: dov smäll som markerar ett drop.
pub fn impact(t: &mut Track, at: f64, gain: f32, rng: &mut Rng) {
    let start = t.frame_at(at);
    let len = (0.9 * t.sr) as usize;
    let mut lp = LowPass::new(500.0, t.sr);
    let mut phase = 0.0f32;
    for i in 0..len {
        let sec = i as f32 / t.sr;
        let amp = (-sec * 5.5).exp();
        let f = 90.0 + 60.0 * (-sec * 12.0).exp();
        phase += f / t.sr;
        if phase > 1.0 {
            phase -= 1.0;
        }
        let low = crate::dsp::sine(phase);
        let noise = lp.process(rng.noise());
        let s = (low * 0.8 + noise * 0.5) * amp * gain;
        t.add(start + i, s, s);
    }
}
