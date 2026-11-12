//! Datamodell for en lat, parsad ur "<titel> - lattext.txt".

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    Intro,
    Verse,
    PreChorus,
    Drop,
    Bridge,
    Outro,
}

impl SectionKind {
    pub fn from_name(name: &str) -> SectionKind {
        let n = name.to_lowercase();
        if n.contains("intro") {
            SectionKind::Intro
        } else if n.contains("pre-chorus") || n.contains("prechorus") || n.contains("pre chorus") {
            SectionKind::PreChorus
        } else if n.contains("drop") || n.contains("refr") {
            SectionKind::Drop
        } else if n.contains("brygga") || n.contains("bridge") {
            SectionKind::Bridge
        } else if n.contains("outro") {
            SectionKind::Outro
        } else {
            SectionKind::Verse
        }
    }
}

/// En ackordtyp vi kan spela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Major,
    Minor,
}

/// Ett ackord: grundton som halvtoner over A (A = 0) och dur/moll.
#[derive(Debug, Clone, Copy)]
pub struct Chord {
    pub root: i32,
    pub quality: Quality,
}

impl Chord {
    /// Halvtoner relativt grundtonen i en triad.
    pub fn intervals(&self) -> [i32; 3] {
        match self.quality {
            Quality::Major => [0, 4, 7],
            Quality::Minor => [0, 3, 7],
        }
    }
}

/// En sektion ur STRUKTUR-tabellen, berikad med ackord och text.
#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub kind: SectionKind,
    pub bars: usize,
    pub start_bar: usize,
    pub start_sec: f64,
    pub chords: Vec<Chord>,
    /// Takter per ackord (1 for vers/drop-loopar, 2 for pre-chorus).
    pub bars_per_chord: usize,
    /// Mixnoter, t.ex. "KICK", "BAS", "RISER" - styr arrangemanget.
    pub tags: Vec<String>,
    /// Textrader (utan mixnoter och rubriker).
    pub lines: Vec<String>,
}

impl Section {
    pub fn end_sec(&self, sec_per_bar: f64) -> f64 {
        self.start_sec + self.bars as f64 * sec_per_bar
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

/// Hela latten.
#[derive(Debug, Clone)]
pub struct Song {
    pub title: String,
    pub genre: String,
    pub bpm: f64,
    pub sec_per_bar: f64,
    pub key_root: i32,
    pub key_minor: bool,
    pub total_bars: usize,
    pub duration: f64,
    pub sections: Vec<Section>,
}
