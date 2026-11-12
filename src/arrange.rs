//! Arrangemang: gör om sektionerna till faktiska slag och noter.

use crate::dsp::Rng;
use crate::model::{SectionKind, Song};
use crate::perc;
use crate::synth;
use crate::wav::Track;

/// Genrejustering av ljudet, utifrån `Genre:`-raden i låttexten.
pub struct Preset {
    pub bas: f32,
    pub kick: f32,
    pub pad: f32,
    pub hats: f32,
    pub kick_topp: f32,
    pub kick_botten: f32,
}

pub fn preset_for(genre: &str) -> Preset {
    let g = genre.to_lowercase();
    if g.contains("dansband") {
        Preset {
            bas: 1.8,
            kick: 1.15,
            pad: 1.4,
            hats: 0.5,
            kick_topp: 110.0,
            kick_botten: 38.0,
        }
    } else if g.contains("techno") {
        Preset {
            bas: 1.0,
            kick: 1.15,
            pad: 0.5,
            hats: 1.3,
            kick_topp: 150.0,
            kick_botten: 52.0,
        }
    } else if g.contains("trap") {
        Preset {
            bas: 1.4,
            kick: 0.85,
            pad: 0.8,
            hats: 1.5,
            kick_topp: 120.0,
            kick_botten: 45.0,
        }
    } else if g.contains("synthwave") {
        Preset {
            bas: 1.1,
            kick: 0.95,
            pad: 1.6,
            hats: 0.7,
            kick_topp: 140.0,
            kick_botten: 55.0,
        }
    } else if g.contains("rock") {
        Preset {
            bas: 1.2,
            kick: 1.2,
            pad: 0.6,
            hats: 1.2,
            kick_topp: 160.0,
            kick_botten: 55.0,
        }
    } else {
        Preset {
            bas: 1.0,
            kick: 1.0,
            pad: 1.0,
            hats: 1.0,
            kick_topp: 143.0,
            kick_botten: 48.0,
        }
    }
}

fn preset_mult(p: &Preset, bus: Bus) -> f32 {
    match bus {
        Bus::Bas => p.bas,
        Bus::Kick => p.kick,
        Bus::Pad => p.pad,
        Bus::Hats => p.hats,
        _ => 1.0,
    }
}

/// Bussens plats i `Settings::buss_gain`.
pub fn idx(bus: Bus) -> usize {
    match bus {
        Bus::Kick => 0,
        Bus::Bas => 1,
        Bus::Ackord => 2,
        Bus::Pad => 3,
        Bus::Hats => 4,
        Bus::Clap => 5,
        Bus::Fx => 6,
        Bus::Lead => 7,
    }
}

/// En buss: allt som har egen volym och kan duckas eller exporteras separat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bus {
    Kick,
    Bas,
    Ackord,
    Pad,
    Hats,
    Clap,
    Fx,
    Lead,
}

pub const BUSSAR: [Bus; 8] = [
    Bus::Kick,
    Bus::Bas,
    Bus::Ackord,
    Bus::Pad,
    Bus::Hats,
    Bus::Clap,
    Bus::Fx,
    Bus::Lead,
];

impl Bus {
    pub fn namn(&self) -> &'static str {
        match self {
            Bus::Kick => "kick",
            Bus::Bas => "bas",
            Bus::Ackord => "ackord",
            Bus::Pad => "pad",
            Bus::Hats => "hats",
            Bus::Clap => "clap",
            Bus::Fx => "fx",
            Bus::Lead => "lead",
        }
    }

    /// Grundvolym per buss.
    pub fn gain(&self) -> f32 {
        match self {
            Bus::Kick => 1.00,
            Bus::Bas => 0.60,
            Bus::Ackord => 0.22,
            Bus::Pad => 0.16,
            Bus::Hats => 0.20,
            Bus::Clap => 0.45,
            Bus::Fx => 0.60,
            Bus::Lead => 0.30,
        }
    }
}

/// Vilka element som spelar i en sektion.
#[derive(Default)]
struct Flags {
    kick: bool,
    bass: bool,
    stab: bool,
    pad: bool,
    hats: bool,
    claps: bool,
    kick_from: usize,
}

fn flags_for(kind: SectionKind) -> Flags {
    let mut f = Flags::default();
    match kind {
        SectionKind::Intro => {
            f.kick = true;
            f.bass = true;
            f.pad = true;
            f.hats = true;
            f.kick_from = 4;
        }
        SectionKind::Verse => {
            f.kick = true;
            f.bass = true;
            f.stab = true;
            f.pad = true;
            f.hats = true;
        }
        SectionKind::PreChorus => {
            f.kick = true;
            f.bass = true;
            f.stab = true;
            f.hats = true;
        }
        SectionKind::Drop => {
            f.kick = true;
            f.bass = true;
            f.stab = true;
            f.hats = true;
            f.claps = true;
        }
        SectionKind::Bridge => {
            f.bass = true;
            f.pad = true;
        }
        SectionKind::Outro => {
            f.bass = true;
            f.pad = true;
        }
    }
    f
}

/// Sektionsvolym.
fn gain_for(kind: SectionKind) -> f32 {
    match kind {
        SectionKind::Intro => 0.55,
        SectionKind::Verse => 0.70,
        SectionKind::PreChorus => 0.80,
        SectionKind::Drop => 1.00,
        SectionKind::Bridge => 0.45,
        SectionKind::Outro => 0.50,
    }
}

use crate::model::{Chord, Quality, Section};

/// Ackordet som gäller i en takt.
fn chord_at(sec: &Section, bar: usize) -> Chord {
    if sec.chords.is_empty() {
        return Chord {
            root: 0,
            quality: Quality::Minor,
        };
    }
    let idx = (bar / sec.bars_per_chord.max(1)) % sec.chords.len();
    sec.chords[idx]
}

/// Ackordets toner i en given oktav, med färgning.
fn chord_freqs(c: Chord, octave: i32, farg: u8) -> Vec<f32> {
    let iv = c.intervals();
    let mut t = vec![iv[0], iv[1], iv[2]];
    let minor = c.quality == Quality::Minor;
    match farg {
        1 => t.push(if minor { 10 } else { 11 }),
        2 => {
            t.push(if minor { 10 } else { 11 });
            t.push(14);
        }
        3 => {
            t[1] = 5;
            t.push(10);
        }
        _ => {}
    }
    t.iter().map(|i| synth::note_hz(c.root + i, octave)).collect()
}

/// Renderar hela låten: en buss i taget, sedan summerat till mastern.
pub fn render(song: &Song, master: &mut Track, s: &crate::settings::Settings) {
    let sek = master.frames as f64 / master.sr as f64;
    let mut send = Track::new(master.sr, sek);
    let p = preset_for(&song.genre);
    for bus in BUSSAR {
        if !bus_pa(bus, s) {
            continue;
        }
        let mut t = Track::new(master.sr, sek);
        render_bus(bus, song, &mut t, s);
        if s.sidechain && matches!(bus, Bus::Bas | Bus::Ackord | Bus::Pad) {
            duck(&mut t, song);
        }
        let bg = s.buss_gain[idx(bus)]
            * preset_mult(&p, bus)
            * if bus == Bus::Lead { s.lead_mangd } else { 1.0 };
        master.mix_from(&t, bg);
        let sa = send_amount(bus);
        if sa > 0.0 {
            send.mix_from(&t, sa * s.fx_mangd);
        }
    }
    if s.reverb || s.delay {
        apply_sends(&send, master, song, s);
    }
}

/// Hur mycket av bussen som går till reverb/delay. Kick och bas går torrt.
fn send_amount(bus: Bus) -> f32 {
    match bus {
        Bus::Pad => 0.7,
        Bus::Ackord => 0.7,
        Bus::Clap => 0.5,
        Bus::Fx => 0.45,
        Bus::Hats => 0.25,
        Bus::Lead => 0.5,
        Bus::Kick | Bus::Bas => 0.0,
    }
}

/// Spelar bussen med nuvarande inställningar?
fn bus_pa(bus: Bus, s: &crate::settings::Settings) -> bool {
    use crate::settings::Element as E;
    match bus {
        Bus::Kick => s.get(E::Kick),
        Bus::Bas => s.get(E::Bas),
        Bus::Ackord => s.get(E::Ackord),
        Bus::Pad => s.get(E::Pad),
        Bus::Hats => s.get(E::Hats),
        Bus::Clap => s.get(E::Clap) || s.get(E::Fill),
        Bus::Fx => s.get(E::Riser) || s.get(E::Impact),
        Bus::Lead => s.get(E::Lead),
    }
}

fn render_bus(bus: Bus, song: &Song, t: &mut Track, s: &crate::settings::Settings) {
    use crate::settings::Element;
    let spb = song.sec_per_bar;
    let beat = spb / 4.0;
    let mut rng = Rng::new(0xD00D_1234 ^ ((bus as u64) + 1) * 7919);
    let pk = preset_for(&song.genre);

    let mut drop_nr = 0usize;
    for sec in song.sections.iter() {
        let f = flags_for(sec.kind);
        let g = gain_for(sec.kind);
        let fade = sec.kind == SectionKind::Outro;
        if sec.kind == SectionKind::Drop {
            drop_nr += 1;
        }

        if bus == Bus::Fx {
            if s.get(Element::Impact) && sec.kind == SectionKind::Drop {
                perc::impact(t, sec.start_sec, 0.9 * g, &mut rng);
            }
            if s.get(Element::Riser) && sec.kind == SectionKind::PreChorus && sec.bars >= 2 {
                let at = sec.start_sec + (sec.bars - 2) as f64 * spb;
                perc::riser(t, at, 2.0 * spb, 0.30, &mut rng);
            }
            continue;
        }

        for bar in 0..sec.bars {
            let bar_t = sec.start_sec + bar as f64 * spb;
            let amp = if fade {
                g * (1.0 - bar as f32 / sec.bars.max(1) as f32)
            } else {
                g
            };
            let ch = chord_at(sec, bar);
            let freqs = chord_freqs(ch, -1, s.farg);
            let sub = synth::note_hz(ch.root, -3);

            match bus {
                Bus::Bas if f.bass => synth::bass(t, bar_t, spb * 0.92, sub, amp),
                Bus::Pad if f.pad => synth::pad(t, bar_t, spb, &freqs, amp),
                Bus::Ackord if f.stab => {
                    synth::stab(t, bar_t, 0.35, &freqs, 0.9 * amp);
                    synth::stab(t, bar_t + spb * 0.75, 0.22, &freqs, 0.6 * amp);
                }
                Bus::Kick => {
                    if f.kick && bar >= f.kick_from {
                        trummor(
                            sec, bus, bar, bar_t, spb, beat, amp, t, &mut rng,
                            pk.kick_topp, pk.kick_botten,
                        );
                    }
                }
                Bus::Hats if f.hats => {
                    trummor(
                        sec, bus, bar, bar_t, spb, beat, amp, t, &mut rng,
                        pk.kick_topp, pk.kick_botten,
                    );
                }
                Bus::Clap => {
                    if s.get(Element::Fill)
                        && sec.kind == SectionKind::PreChorus
                        && bar + 1 == sec.bars
                    {
                        perc::fill(t, bar_t, spb, 0.45 * amp, &mut rng);
                    } else if f.claps {
                        trummor(
                            sec, bus, bar, bar_t, spb, beat, amp, t, &mut rng,
                            pk.kick_topp, pk.kick_botten,
                        );
                    }
                }
                Bus::Lead => {
                    if sec.kind == SectionKind::Drop {
                        let steg = if drop_nr >= 2 { 8 } else { 4 };
                        let dur = spb / steg as f64 * 0.9;
                        let fq = synth::note_hz(ch.root, 0);
                        for k in 0..steg {
                            let at = bar_t + k as f64 * spb / steg as f64;
                            synth::pluck(t, at, dur, fq, amp * (1.0 - 0.35 * k as f32 / steg as f32));
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn trummor(
    sec: &Section,
    bus: Bus,
    bar: usize,
    bar_t: f64,
    spb: f64,
    beat: f64,
    amp: f32,
    t: &mut Track,
    rng: &mut Rng,
    kick_topp: f32,
    kick_botten: f32,
) {
    let gap = sec.kind == SectionKind::PreChorus && bar + 2 >= sec.bars;
    if bus == Bus::Kick {
        if sec.kind == SectionKind::Bridge {
            synth::kick_djup(t, bar_t, 0.85 * amp, kick_topp, kick_botten);
            return;
        }
        if gap {
            return;
        }
        for b in 0..4 {
            synth::kick_djup(t, bar_t + b as f64 * beat, 0.95 * amp, kick_topp, kick_botten);
        }
        return;
    }
    if bus == Bus::Hats {
        let div = if sec.kind == SectionKind::Drop { 4 } else { 2 };
        let step = spb / div as f64;
        for i in 0..div {
            perc::hat(t, bar_t + i as f64 * step, 0.20 * amp, rng, i == div - 1);
        }
        return;
    }
    if bus == Bus::Clap {
        perc::clap(t, bar_t + beat, 0.45 * amp, rng);
        perc::clap(t, bar_t + 3.0 * beat, 0.45 * amp, rng);
    }
}

/// Skickar send-bufferten genom reverb och delay in i mastern.
fn apply_sends(send: &Track, master: &mut Track, song: &Song, s: &crate::settings::Settings) {
    let (rev, del) = berakna_returer(send, song, s);
    master.mix_from(&rev, 1.0);
    master.mix_from(&del, 1.0);
}

/// Räknar ut de våta returerna (reverb, delay) ur send-bufferten.
fn berakna_returer(send: &Track, song: &Song, s: &crate::settings::Settings) -> (Track, Track) {
    let sr = send.sr;
    let sek = send.frames as f64 / sr as f64;
    let mut rev_l = crate::fx::Reverb::new(sr, 1.5);
    let mut rev_r = crate::fx::Reverb::new(sr, 1.9);
    let mut del = crate::fx::Delay::new(sr, (song.sec_per_bar * 0.75) as f32, 0.34);
    let mut wet_rev = Track::new(sr, sek);
    let mut wet_del = Track::new(sr, sek);

    for f in 0..send.frames {
        let x = (send.data[f * 2] + send.data[f * 2 + 1]) * 0.5;
        if s.reverb {
            wet_rev.data[f * 2] = rev_l.process(x) * 0.75;
            wet_rev.data[f * 2 + 1] = rev_r.process(x) * 0.75;
        }
        if s.delay {
            let (dl, dr) = del.process(x);
            wet_del.data[f * 2] = dl * 0.35;
            wet_del.data[f * 2 + 1] = dr * 0.35;
        }
    }
    (wet_rev, wet_del)
}

/// Skriver ett stem per buss (plus effektreturer) till `dir`.
pub fn exportera_stems(song: &Song, s: &crate::settings::Settings, dir: &std::path::Path) -> Result<Vec<String>, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("kunde inte skapa {}: {e}", dir.display()))?;
    let (sr, sek) = (crate::SR, song.duration + 2.0);
    let mut send = Track::new(sr, sek);
    let mut namn = Vec::new();
    let p = preset_for(&song.genre);

    for bus in BUSSAR {
        if !bus_pa(bus, s) { continue; }
        let mut t = Track::new(sr, sek);
        render_bus(bus, song, &mut t, s);
        if s.sidechain && matches!(bus, Bus::Bas | Bus::Ackord | Bus::Pad) { duck(&mut t, song); }
        let sa = send_amount(bus);
        if sa > 0.0 { send.mix_from(&t, sa * s.fx_mangd); }
        let bg = s.buss_gain[idx(bus)]
            * preset_mult(&p, bus)
            * if bus == Bus::Lead { s.lead_mangd } else { 1.0 };
        for v in t.data.iter_mut() { *v *= bg; }
        let fil = format!("{}.wav", bus.namn());
        crate::wav::write_wav(&dir.join(&fil), &t).map_err(|e| format!("{fil}: {e}"))?;
        namn.push(fil);
    }

    if s.reverb || s.delay {
        let (rev, del) = berakna_returer(&send, song, s);
        if s.reverb { skriv(dir, "reverb.wav", &rev, &mut namn)?; }
        if s.delay { skriv(dir, "delay.wav", &del, &mut namn)?; }
    }
    Ok(namn)
}

fn skriv(dir: &std::path::Path, fil: &str, t: &Track, ut: &mut Vec<String>) -> Result<(), String> {
    crate::wav::write_wav(&dir.join(fil), t).map_err(|e| format!("{fil}: {e}"))?;
    ut.push(fil.to_string());
    Ok(())
}

/// Sidechain: kicken duckar bas, ackord och pad vid varje slag.
fn duck(t: &mut Track, song: &Song) {
    let spb = song.sec_per_bar;
    let beat = spb / 4.0;
    for sec in &song.sections {
        if matches!(sec.kind, SectionKind::Bridge | SectionKind::Outro) {
            continue;
        }
        for bar in 0..sec.bars {
            if sec.kind == SectionKind::PreChorus && bar + 2 >= sec.bars {
                continue;
            }
            for b in 0..4 {
                let at = sec.start_sec + bar as f64 * spb + b as f64 * beat;
                duck_at(t, at);
            }
        }
    }
}

fn duck_at(t: &mut Track, at: f64) {
    const AMOUNT: f32 = 0.55;
    const RELEASE: f64 = 0.22;
    let start = t.frame_at(at);
    let len = (RELEASE * t.sr as f64) as usize;
    for i in 0..len {
        let f = start + i;
        if f >= t.frames {
            break;
        }
        let p = i as f32 / len as f32;
        let g = 1.0 - AMOUNT * (1.0 - p) * (1.0 - p);
        t.data[f * 2] *= g;
        t.data[f * 2 + 1] *= g;
    }
}
