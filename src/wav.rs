//! Stereobuffer och WAV-export (16-bitars PCM).

use std::fs::File;
use std::io::{BufWriter, Result, Write};
use std::path::Path;

pub struct Track {
    pub sr: f32,
    pub frames: usize,
    /// Interleaved stereo: L, R, L, R ...
    pub data: Vec<f32>,
}

impl Track {
    pub fn new(sr: f32, seconds: f64) -> Track {
        let frames = (sr as f64 * seconds) as usize + 1;
        Track {
            sr,
            frames,
            data: vec![0.0; frames * 2],
        }
    }

    /// Lägger till en stereosample vid en frame-position.
    pub fn add(&mut self, frame: usize, l: f32, r: f32) {
        if frame >= self.frames {
            return;
        }
        self.data[frame * 2] += l;
        self.data[frame * 2 + 1] += r;
    }

    pub fn frame_at(&self, seconds: f64) -> usize {
        (seconds * self.sr as f64) as usize
    }

    /// Lägger en annan buffer ovanpå denna, med en gain.
    pub fn mix_from(&mut self, other: &Track, gain: f32) {
        let n = self.data.len().min(other.data.len());
        for i in 0..n {
            self.data[i] += other.data[i] * gain;
        }
    }

    /// Normaliserar om toppen överstiger `ceiling`.
    pub fn normalize(&mut self, ceiling: f32) {
        let peak = self
            .data
            .iter()
            .fold(0.0f32, |m, v| if v.abs() > m { v.abs() } else { m });
        if peak > ceiling && peak > 0.0 {
            let g = ceiling / peak;
            for v in self.data.iter_mut() {
                *v *= g;
            }
        }
    }

    /// Mäter RMS i dBFS (närmevärde för loudness).
    pub fn rms_db(&self) -> f32 {
        let n = self.data.len().max(1);
        let sum: f64 = self.data.iter().map(|v| (*v as f64) * (*v as f64)).sum();
        let rms = (sum / n as f64).sqrt() as f32;
        20.0 * (rms + 1e-9).log10()
    }

    pub fn peak_db(&self) -> f32 {
        let peak = self.data.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        20.0 * (peak + 1e-9).log10()
    }

    /// Loudness-normalisering: gainas till en målnivå och mjukbegränsas mot ett tak.
    /// Till skillnad från `normalize` sänks inte hela mixen av en enstaka topp.
    pub fn loudness(&mut self, target_db: f32, ceiling_db: f32) {
        let rms = 10f32.powf(self.rms_db() / 20.0);
        if rms <= 1e-9 {
            return;
        }
        let g = 10f32.powf(target_db / 20.0) / rms;
        let ceil = 10f32.powf(ceiling_db / 20.0);
        let knee = ceil * 0.6;

        for v in self.data.iter_mut() {
            let x = *v * g;
            *v = if x.abs() <= knee {
                x
            } else {
                let over = x.abs() - knee;
                let span = ceil - knee;
                x.signum() * (knee + span * (over / (over + span)))
            };
        }
    }
}

pub fn write_wav(path: &Path, track: &Track) -> Result<()> {
    let mut w = BufWriter::new(File::create(path)?);
    let channels: u16 = 2;
    let bits: u16 = 16;
    let bytes_per_sample = (bits / 8) as u32;
    let data_len = (track.frames as u32) * (channels as u32) * bytes_per_sample;

    w.write_all(b"RIFF")?;
    w.write_all(&(36 + data_len).to_le_bytes())?;
    w.write_all(b"WAVE")?;
    w.write_all(b"fmt ")?;
    w.write_all(&16u32.to_le_bytes())?;
    w.write_all(&1u16.to_le_bytes())?; // PCM
    w.write_all(&channels.to_le_bytes())?;
    w.write_all(&(track.sr as u32).to_le_bytes())?;
    let byte_rate = track.sr as u32 * channels as u32 * bytes_per_sample;
    w.write_all(&byte_rate.to_le_bytes())?;
    w.write_all(&((channels * bits / 8) as u16).to_le_bytes())?;
    w.write_all(&bits.to_le_bytes())?;
    w.write_all(b"data")?;
    w.write_all(&data_len.to_le_bytes())?;

    for v in &track.data {
        let s = (v.clamp(-1.0, 1.0) * 32767.0) as i16;
        w.write_all(&s.to_le_bytes())?;
    }
    w.flush()
}
