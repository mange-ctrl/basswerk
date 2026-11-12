//! Produktionsnivå och elementval.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Element {
    Kick,
    Bas,
    Ackord,
    Pad,
    Hats,
    Clap,
    Riser,
    Impact,
    Sidechain,
    Reverb,
    Delay,
    Fill,
    Lead,
}

pub const ALLA: [Element; 13] = [
    Element::Kick,
    Element::Bas,
    Element::Ackord,
    Element::Pad,
    Element::Hats,
    Element::Clap,
    Element::Riser,
    Element::Impact,
    Element::Sidechain,
    Element::Reverb,
    Element::Delay,
    Element::Fill,
    Element::Lead,
];

impl Element {
    pub fn namn(&self) -> &'static str {
        match self {
            Element::Kick => "Kick",
            Element::Bas => "Bas",
            Element::Ackord => "Ackord",
            Element::Pad => "Pad",
            Element::Hats => "Hi-hats",
            Element::Clap => "Clap",
            Element::Riser => "Riser",
            Element::Impact => "Impact",
            Element::Sidechain => "Sidechain",
            Element::Reverb => "Reverb",
            Element::Delay => "Delay",
            Element::Fill => "Fills",
            Element::Lead => "Lead",
        }
    }

    /// Formatet som sparas i installning.txt.
    pub fn nyckel(&self) -> &'static str {
        match self {
            Element::Kick => "kick",
            Element::Bas => "bas",
            Element::Ackord => "ackord",
            Element::Pad => "pad",
            Element::Hats => "hats",
            Element::Clap => "clap",
            Element::Riser => "riser",
            Element::Impact => "impact",
            Element::Sidechain => "sidechain",
            Element::Reverb => "reverb",
            Element::Delay => "delay",
            Element::Fill => "fill",
            Element::Lead => "lead",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub niva: u8,
    pub kick: bool,
    pub bas: bool,
    pub ackord: bool,
    pub pad: bool,
    pub hats: bool,
    pub clap: bool,
    pub riser: bool,
    pub impact: bool,
    pub sidechain: bool,
    pub reverb: bool,
    pub delay: bool,
    /// Hur mycket reverb/delay som skickas in. 0.0 = torrt, 1.0 = fullt.
    pub fx_mangd: f32,
    pub fill: bool,
    pub lead: bool,
    /// Volym för lead-lagret. 0.0 = tyst, 1.0 = fullt.
    pub lead_mangd: f32,
    /// Exportera ett WAV per buss.
    pub stems: bool,
    /// Volym per buss: kick, bas, ackord, pad, hats, clap, fx, lead.
    pub buss_gain: [f32; 8],
    /// Ackordfärgning: 0 = treklang, 1 = septima, 2 = + nia, 3 = sus.
    pub farg: u8,
}

pub fn buss_namn() -> [&'static str; 8] {
    ["kick", "bas", "ackord", "pad", "hats", "clap", "fx", "lead"]
}

pub fn buss_default() -> [f32; 8] {
    [1.0, 0.60, 0.22, 0.16, 0.20, 0.45, 0.60, 0.30]
}

/// App-inställningar: gäller alla låtar, sparas i %APPDATA%\basswerk.
#[derive(Debug, Clone)]
pub struct AppSettings {
    pub arbetsmapp: String,
    pub modell: String,
    pub opencode_sokvag: String,
    pub genre: String,
    pub textniva: u8,
}

/// Valbara genrer. Styr bara texten (skickas med i prompten).
pub const GENRER: [&str; 6] = [
    "Raggardunk",
    "Dansband",
    "Techno",
    "Trap",
    "Synthwave",
    "Hårdrock",
];

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            arbetsmapp: String::new(),
            modell: "opencode/deepseek-v4.1-flash-free".into(),
            opencode_sokvag: String::new(),
            genre: "Raggardunk".into(),
            textniva: 3,
        }
    }
}

impl AppSettings {
    pub fn till_text(&self) -> String {
        format!(
            "# basswerk app-inställningar\narbetsmapp={}\nmodell={}\nopencode={}\ngenre={}\ntextniva={}\n",
            self.arbetsmapp, self.modell, self.opencode_sokvag, self.genre, self.textniva
        )
    }

    pub fn fran_text(t: &str) -> AppSettings {
        let mut a = AppSettings::default();
        for rad in t.lines() {
            let rad = rad.trim();
            if rad.is_empty() || rad.starts_with('#') {
                continue;
            }
            let Some((k, v)) = rad.split_once('=') else { continue };
            let v = v.trim();
            match k.trim() {
                "arbetsmapp" => a.arbetsmapp = v.to_string(),
                "modell" => a.modell = v.to_string(),
                "opencode" => a.opencode_sokvag = v.to_string(),
                "genre" => a.genre = v.to_string(),
                "textniva" => {
                    if let Ok(n) = v.parse() {
                        a.textniva = n;
                    }
                }
                _ => {}
            }
        }
        a
    }
}

/// Var app-inställningarna bor.
pub fn app_sokvag() -> PathBuf {
    let bas = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    bas.join("basswerk").join("installning.txt")
}

pub fn las_app() -> AppSettings {
    std::fs::read_to_string(app_sokvag())
        .ok()
        .map(|t| AppSettings::fran_text(&t))
        .unwrap_or_default()
}

pub fn spara_app(a: &AppSettings) -> std::io::Result<()> {
    let p = app_sokvag();
    if let Some(par) = p.parent() {
        std::fs::create_dir_all(par)?;
    }
    std::fs::write(&p, a.till_text())
}

impl Default for Settings {
    fn default() -> Self {
        Settings::preset(3)
    }
}

impl Settings {
    pub fn preset(niva: u8) -> Settings {
        let mut s = Settings {
            niva,
            kick: true,
            bas: true,
            ackord: false,
            pad: false,
            hats: false,
            clap: false,
            riser: false,
            impact: false,
            sidechain: false,
            reverb: false,
            delay: false,
            fx_mangd: 0.75,
            fill: false,
            lead: false,
            lead_mangd: 0.35,
            stems: false,
            buss_gain: buss_default(),
            farg: 0,
        };
        let n = niva.clamp(1, 5);
        if n >= 2 {
            s.hats = true;
        }
        if n >= 3 {
            s.ackord = true;
            s.pad = true;
        }
        if n >= 4 {
            s.clap = true;
            s.riser = true;
            s.impact = true;
            s.sidechain = true;
        }
        if n >= 5 {
            s.reverb = true;
            s.delay = true;
            s.fill = true;
            s.lead = true;
        }
        s
    }

    pub fn get(&self, e: Element) -> bool {
        match e {
            Element::Kick => self.kick,
            Element::Bas => self.bas,
            Element::Ackord => self.ackord,
            Element::Pad => self.pad,
            Element::Hats => self.hats,
            Element::Clap => self.clap,
            Element::Riser => self.riser,
            Element::Impact => self.impact,
            Element::Sidechain => self.sidechain,
            Element::Reverb => self.reverb,
            Element::Delay => self.delay,
            Element::Fill => self.fill,
            Element::Lead => self.lead,
        }
    }

    /// Sätter ett element och tappar preset-kopplingen.
    pub fn set(&mut self, e: Element, v: bool) {
        match e {
            Element::Kick => self.kick = v,
            Element::Bas => self.bas = v,
            Element::Ackord => self.ackord = v,
            Element::Pad => self.pad = v,
            Element::Hats => self.hats = v,
            Element::Clap => self.clap = v,
            Element::Riser => self.riser = v,
            Element::Impact => self.impact = v,
            Element::Sidechain => self.sidechain = v,
            Element::Reverb => self.reverb = v,
            Element::Delay => self.delay = v,
            Element::Fill => self.fill = v,
            Element::Lead => self.lead = v,
        }
        self.niva = 0;
    }

    pub fn niva_namn(&self) -> String {
        let namn = match self.niva {
            1 => "Skiss",
            2 => "Grund",
            3 => "Full",
            4 => "Prod",
            5 => "Max",
            _ => "Egen",
        };
        if self.niva == 0 {
            namn.to_string()
        } else {
            format!("{} - {}", self.niva, namn)
        }
    }

    pub fn till_text(&self) -> String {
        let mut s = format!(
            "# basswerk-inställningar\nniva={}\nfx={:.2}\nleadmangd={:.2}\nstems={}\n",
            self.niva,
            self.fx_mangd,
            self.lead_mangd,
            if self.stems { 1 } else { 0 }
        );
        for e in ALLA {
            let _ = writeln!(s, "{}={}", e.nyckel(), if self.get(e) { 1 } else { 0 });
        }
        for (i, n) in buss_namn().iter().enumerate() {
            let _ = writeln!(s, "gain_{}={:.2}", n, self.buss_gain[i]);
        }
        let _ = writeln!(s, "farg={}", self.farg);
        s
    }

    pub fn fran_text(t: &str) -> Settings {
        let mut s = Settings::preset(3);
        let mut niva: Option<u8> = None;
        for rad in t.lines() {
            let rad = rad.trim();
            if rad.is_empty() || rad.starts_with('#') {
                continue;
            }
            let Some((k, v)) = rad.split_once('=') else { continue };
            let k = k.trim();
            let v = v.trim();
            if k == "niva" {
                niva = v.parse().ok();
                continue;
            }
            if k == "fx" {
                if let Ok(x) = v.replace(',', ".").parse::<f32>() {
                    s.fx_mangd = x;
                }
                continue;
            }
            if k == "leadmangd" {
                if let Ok(x) = v.replace(',', ".").parse::<f32>() {
                    s.lead_mangd = x;
                }
                continue;
            }
            if k == "stems" {
                s.stems = v == "1" || v.eq_ignore_ascii_case("true");
                continue;
            }
            if k == "farg" {
                if let Ok(n) = v.parse() {
                    s.farg = n;
                }
                continue;
            }
            if let Some(bn) = k.strip_prefix("gain_") {
                if let (Some(i), Ok(x)) = (
                    buss_namn().iter().position(|n| *n == bn),
                    v.replace(',', ".").parse::<f32>(),
                ) {
                    s.buss_gain[i] = x;
                }
                continue;
            }
            let b = v == "1" || v.eq_ignore_ascii_case("true");
            for e in ALLA {
                if e.nyckel() == k {
                    s.set(e, b);
                }
            }
        }
        s.niva = niva.unwrap_or(0);
        s
    }
}

/// Var inställningarna sparas för en låt.
pub fn sokvag(dir: &Path) -> PathBuf {
    dir.join("ljud").join("installning.txt")
}

pub fn spara(dir: &Path, s: &Settings) -> std::io::Result<()> {
    let p = sokvag(dir);
    if let Some(par) = p.parent() {
        std::fs::create_dir_all(par)?;
    }
    std::fs::write(&p, s.till_text())
}

pub fn las(dir: &Path) -> Option<Settings> {
    std::fs::read_to_string(sokvag(dir)).ok().map(|t| Settings::fran_text(&t))
}
