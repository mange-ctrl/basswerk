//! Tonhöjdshjälp och melodiska instrument (kick, bas, stabbar, pad).

use crate::dsp::{saw, sine, LowPass};
use crate::wav::Track;

/// Frekvens för en MIDI-not.
pub fn midi_hz(midi: f32) -> f32 {
    440.0 * ((midi - 69.0) / 12.0).exp2()
}

/// Frekvens för en halvton över A, i en given oktav (A4 = oktav 0).
pub fn note_hz(semitone_over_a: i32, octave: i32) -> f32 {
    midi_hz(69.0 + semitone_over_a as f32 + 12.0 * octave as f32)
}

/// Bastrumma: sinus med fallande tonhöjd och snabb avklingning.
pub fn kick(t: &mut Track, at: f64, gain: f32) {
    kick_djup(t, at, gain, 143.0, 48.0)
}

/// Bastrumma med ställbar tonhöjd: `topp` Hz som faller till `botten` Hz.
pub fn kick_djup(t: &mut Track, at: f64, gain: f32, topp: f32, botten: f32) {
    let start = t.frame_at(at);
    let len = (0.30 * t.sr) as usize;
    let mut phase = 0.0f32;
    for i in 0..len {
        let sec = i as f32 / t.sr;
        let f = botten + (topp - botten) * (-sec * 26.0).exp();
        phase += f / t.sr;
        if phase > 1.0 {
            phase -= 1.0;
        }
        let amp = (-sec * 8.5).exp();
        let s = sine(phase) * amp * gain;
        t.add(start + i, s, s);
    }
}

/// Subbas: ren sinus med mjuk attack.
pub fn bass(t: &mut Track, at: f64, dur: f64, freq: f32, gain: f32) {
    let start = t.frame_at(at);
    let len = (dur * t.sr as f64) as usize;
    let mut phase = 0.0f32;
    for i in 0..len {
        let sec = i as f32 / t.sr;
        let attack = (sec * 90.0).min(1.0);
        let amp = attack * (-sec * 2.2).exp();
        phase += freq / t.sr;
        if phase > 1.0 {
            phase -= 1.0;
        }
        let s = sine(phase) * amp * gain;
        t.add(start + i, s, s);
    }
}

/// Ackordstabbe: tre sågtänder med lätt detune, lågpassade.
pub fn stab(t: &mut Track, at: f64, dur: f64, freqs: &[f32], gain: f32) {
    let start = t.frame_at(at);
    let len = (dur * t.sr as f64) as usize;
    let detune = [0.997, 1.0, 1.004];
    for i in 0..len {
        let sec = i as f32 / t.sr;
        let amp = (sec * 260.0).min(1.0) * (-sec * 4.5).exp();
        let mut sum = 0.0f32;
        for (k, f) in freqs.iter().enumerate() {
            let d = detune[k % detune.len()];
            let ph = (sec * f * d) % 1.0;
            sum += saw(ph);
        }
        let s = sum / (freqs.len() as f32).sqrt() * amp * gain;
        t.add(start + i, s, s);
    }
}

/// Plock: kort ton för lead-lagret, lite vassare än stabben.
pub fn pluck(t: &mut Track, at: f64, dur: f64, freq: f32, gain: f32) {
    let start = t.frame_at(at);
    let len = (dur * t.sr as f64) as usize;
    let mut lp = LowPass::new(2200.0, t.sr);
    for i in 0..len {
        let sec = i as f32 / t.sr;
        let amp = (sec * 400.0).min(1.0) * (-sec * 9.0).exp();
        let ph = (sec * freq) % 1.0;
        let s = lp.process(saw(ph) * 0.6 + sine(ph * 2.0) * 0.4);
        t.add(start + i, s * amp * gain, s * amp * gain);
    }
}

/// Pad: mjuk, långsam och dov.
pub fn pad(t: &mut Track, at: f64, dur: f64, freqs: &[f32], gain: f32) {
    let start = t.frame_at(at);;
    let len = (dur * t.sr as f64) as usize;
    let mut lp = LowPass::new(900.0, t.sr);
    for i in 0..len {
        let sec = i as f32 / t.sr;
        let fade_in = (sec / 0.6).min(1.0);
        let fade_out = ((dur as f32 - sec) / 0.8).clamp(0.0, 1.0);
        let mut sum = 0.0f32;
        for f in freqs {
            sum += sine((sec * f) % 1.0);
        }
        let s = lp.process(sum / (freqs.len() as f32).sqrt()) * fade_in * fade_out * gain;
        t.add(start + i, s, s);
    }
}
