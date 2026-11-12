//! Läser en låtmapp och bygger en `Song`.

use crate::header;
use crate::model::Song;
use crate::sections;
use std::fs;
use std::path::{Path, PathBuf};

/// Letar upp allt-i-ett-filen: "<titel> - låttext.txt".
pub fn find_song_file(dir: &Path) -> Option<PathBuf> {
    let mut fallback: Option<PathBuf> = None;
    for e in fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        let name = p.file_name()?.to_string_lossy().to_lowercase();
        if !name.ends_with(".txt") {
            continue;
        }
        if name.contains("låttext") || name.contains("lattext") {
            return Some(p);
        }
        fallback = Some(p);
    }
    fallback
}

/// Läser filen och bygger en `Song`.
pub fn load(path: &Path) -> Result<Song, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("kunde inte läsa {}: {e}", path.display()))?;
    let lines: Vec<&str> = text.lines().collect();

    let h = header::parse(&lines);
    let secs = sections::parse(&lines);
    if secs.is_empty() {
        return Err(format!("hittade inga sektioner i {}", path.display()));
    }

    let total_bars: usize = secs.iter().map(|s| s.bars).sum();
    let computed = total_bars as f64 * h.sec_per_bar;
    let duration = if h.duration > 0.0 { h.duration } else { computed };

    Ok(Song {
        title: h.title,
        genre: h.genre,
        bpm: h.bpm,
        sec_per_bar: h.sec_per_bar,
        key_root: h.key_root,
        key_minor: h.key_minor,
        total_bars,
        duration,
        sections: secs,
    })
}
