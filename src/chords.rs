//! Tonnamn och ackordtolkning.

use crate::model::{Chord, Quality};

/// Halvtoner räknat från A: A=0, A#=1, B=2, C=3, C#=4, D=5, D#=6, E=7, F=8, F#=9, G=10, G#=11.
pub const NOTES: [&str; 12] = [
    "A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#",
];

/// Tolkar ett tonnamn, t.ex. "Em", "C" eller "F#".
pub fn parse_note(token: &str) -> Option<(i32, bool)> {
    let t = token.trim();
    let mut chars = t.chars();
    let letter = chars.next()?.to_ascii_uppercase();
    if !('A'..='G').contains(&letter) {
        return None;
    }
    let mut name = String::new();
    name.push(letter);
    let rest: String = chars.collect();
    if rest.starts_with('#') {
        name.push('#');
    }
    let minor = rest.contains('m') || t.contains("moll");
    let idx = NOTES.iter().position(|n| *n == name)?;
    Some((idx as i32, minor))
}

/// Tolkar ett enskilt ackord, t.ex. "Em".
pub fn parse_chord(token: &str) -> Option<Chord> {
    let (root, minor) = parse_note(token)?;
    Some(Chord {
        root,
        quality: if minor { Quality::Minor } else { Quality::Major },
    })
}

/// Delar en ackordrad i enskilda ackord. Hanterar både "Em - C - G" och "Em Em C C".
pub fn split_chords(part: &str) -> Vec<Chord> {
    let cleaned = match part.find('(') {
        Some(i) => &part[..i],
        None => part,
    };
    cleaned
        .replace('-', " ")
        .split_whitespace()
        .filter_map(parse_chord)
        .collect()
}

/// Första heltalet i en sträng.
pub fn first_int(s: &str) -> Option<i64> {
    let mut num = String::new();
    let mut started = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            num.push(c);
            started = true;
        } else if started {
            break;
        }
    }
    num.parse().ok()
}

/// Hittar heltalet som står omedelbart före `marker` i strängen.
pub fn int_before(s: &str, marker: &str) -> Option<usize> {
    let i = s.find(marker)?;
    let before = s[..i].trim_end();
    let digits: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.chars().rev().collect::<String>().parse().ok()
}

/// Läser "N takter/ackord" ur en ackordrad. Standard 1.
pub fn bars_per_chord(part: &str) -> usize {
    if part.contains("takt/ackord") || part.contains("takter/ackord") {
        if let Some(n) = int_before(part, "takt") {
            if n > 0 {
                return n;
            }
        }
    }
    1
}
