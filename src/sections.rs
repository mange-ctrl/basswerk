//! Delar upp filen i sektioner och läser ackord, mixnoter och text.

use crate::header::mmss;

/// Kända mixnoter-taggar.
pub const TAGS: [&str; 7] = ["BAS", "KICK", "SIDE", "FILT", "RISER", "VOX", "FX"];

pub fn is_section_header(t: &str) -> bool {
    t.starts_with('[')
        && t.contains(']')
        && t.contains('|')
        && crate::chords::int_before(t, "takter").is_some()
}

pub fn name_of(t: &str) -> String {
    match (t.find('['), t.find(']')) {
        (Some(a), Some(b)) if b > a => t[a + 1..b].trim().to_string(),
        _ => String::new(),
    }
}

pub fn start_sec_of(t: &str) -> f64 {
    if let Some(i) = t.find('|') {
        let rest = &t[i + 1..];
        if let Some(tc) = rest.split('-').next() {
            if let Some(s) = mmss(tc.trim()) {
                return s;
            }
        }
    }
    0.0
}

/// Ger taggnamnet om raden är en mixnot, t.ex. "[KICK]  Fyra på golvet...".
pub fn tag_of(t: &str) -> Option<String> {
    let b = t.find(']')?;
    let inner = t[..b].strip_prefix('[')?.trim();
    if TAGS.contains(&inner) {
        Some(inner.to_string())
    } else {
        None
    }
}

/// Städar en textrad. Hoppar över etiketter som "Vers 1:".
pub fn clean_line(t: &str) -> String {
    let s = t.trim();
    if s.ends_with(':') && s.len() < 30 {
        return String::new();
    }
    s.to_string()
}

use crate::chords::{bars_per_chord, first_int, split_chords};
use crate::model::{Section, SectionKind};

/// Läser alla sektionsblock ur filen.
pub fn parse(lines: &[&str]) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    let mut i = 0;
    let mut running_bar = 0usize;

    while i < lines.len() {
        if !is_section_header(lines[i].trim()) {
            i += 1;
            continue;
        }
        let head = lines[i].trim();
        let name = name_of(head);
        let bars = crate::chords::int_before(head, "takter").unwrap_or(0);
        let start_sec = start_sec_of(head);
        let start_bar = running_bar;
        running_bar += bars;

        let mut chords = Vec::new();
        let mut bpc = 1usize;
        let mut tags: Vec<String> = Vec::new();
        let mut lyric: Vec<String> = Vec::new();
        let mut in_text = false;

        let mut j = i + 1;
        while j < lines.len() && !is_section_header(lines[j].trim()) {
            let bt = lines[j].trim();
            if let Some(body) = bt.strip_prefix("Ackord:") {
                chords = split_chords(body);
                bpc = bars_per_chord(body);
            } else if bt.starts_with("TEXT") {
                in_text = true;
            } else if let Some(tag) = tag_of(bt) {
                if !tags.contains(&tag) {
                    tags.push(tag);
                }
            } else if in_text {
                let l = clean_line(bt);
                if !l.is_empty() {
                    lyric.push(l);
                }
            }
            j += 1;
        }

        out.push(Section {
            name: name.clone(),
            kind: SectionKind::from_name(&name),
            bars,
            start_bar,
            start_sec,
            chords,
            bars_per_chord: bpc,
            tags,
            lines: lyric,
        });
        i = j;
    }
    out
}
