//! Läser header-blocket: titel, genre, tempo, tonart, längd.

use crate::chords::{first_int, parse_note};

#[derive(Debug, Default)]
pub struct Header {
    pub title: String,
    pub genre: String,
    pub bpm: f64,
    pub sec_per_bar: f64,
    pub key_root: i32,
    pub key_minor: bool,
    pub total_bars: usize,
    pub duration: f64,
}

/// Tolkar "3:39" till sekunder.
pub fn mmss(s: &str) -> Option<f64> {
    let (m, sec) = s.trim().split_once(':')?;
    let m: f64 = m.trim().parse().ok()?;
    let sec: f64 = sec.trim().replace(',', ".").parse().ok()?;
    Some(m * 60.0 + sec)
}

pub fn parse(lines: &[&str]) -> Header {
    let mut h = Header::default();

    // Titel: första raden i filen som inte är tom eller en separator.
    for l in lines {
        let t = l.trim();
        if !t.is_empty() && !t.starts_with('=') && !t.starts_with("TEMA") {
            if !t.starts_with("Genre:") {
                h.title = t.to_string();
            }
            break;
        }
    }

    for l in lines {
        let t = l.trim();
        if let Some(v) = t.strip_prefix("Genre:") {
            h.genre = v.trim().to_string();
        } else if let Some(v) = t.strip_prefix("TEMPO:") {
            h.bpm = v
                .split_whitespace()
                .next()
                .and_then(|n| n.replace(',', ".").parse().ok())
                .unwrap_or(0.0);
        } else if let Some(v) = t.strip_prefix("LÄNGD PER TAKT:") {
            h.sec_per_bar = v
                .split_whitespace()
                .next()
                .and_then(|n| n.replace(',', ".").parse().ok())
                .unwrap_or(0.0);
        } else if let Some(v) = t.strip_prefix("KEY:") {
            if let Some((root, minor)) = parse_note(v.trim()) {
                h.key_root = root;
                h.key_minor = minor;
            }
        } else if let Some(v) = t.strip_prefix("TOTAL LÄNGD:") {
            if let Some(open) = v.find('(') {
                if let Some(n) = first_int(&v[open..]) {
                    h.total_bars = n as usize;
                }
            }
            if let Some(colon) = v.find('~') {
                if let Some(d) = mmss(&v[colon + 1..].trim().split_whitespace().next().unwrap_or("")) {
                    h.duration = d;
                }
            }
        }
    }

    if h.sec_per_bar == 0.0 && h.bpm > 0.0 {
        h.sec_per_bar = 240.0 / h.bpm;
    }
    h
}
