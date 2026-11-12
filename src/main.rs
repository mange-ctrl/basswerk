//! basswerk - läser en låttext från latstruktur-skillen och genererar musiken.

mod arrange;
mod chords;
mod dsp;
mod fx;
mod gui;
mod header;
mod load;
mod model;
mod perc;
mod sections;
mod settings;
mod synth;
mod wav;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use wav::Track;

pub(crate) const SR: f32 = 44100.0;

const USAGE: &str = "\
basswerk - genererar musik ur en lattext-fil

ANVANDNING:
    basswerk <latmapp> [--out <fil.wav>]

EXEMPEL:
    basswerk \"latar\\niva3-4\\HENNES MUN\"
    basswerk \"latar\\niva5\\TUNGAN\" --out TUNGAN.wav

Laser \"<titel> - lattext.txt\" och skriver en WAV i samma mapp.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return match gui::run(None) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("fel: {e}");
                ExitCode::FAILURE
            }
        };
    }
    if args[0] == "--gui" {
        let mapp = args.get(1).map(PathBuf::from);
        return match gui::run(mapp) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("fel: {e}");
                ExitCode::FAILURE
            }
        };
    }
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fel: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let dir = PathBuf::from(&args[0]);
    if !dir.is_dir() {
        return Err(format!("{} är ingen mapp", dir.display()));
    }

    let mut out: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--out" && i + 1 < args.len() {
            out = Some(PathBuf::from(&args[i + 1]));
            i += 2;
        } else {
            i += 1;
        }
    }

    let file = load::find_song_file(&dir)
        .ok_or_else(|| format!("hittade ingen .txt i {}", dir.display()))?;
    let song = load::load(&file)?;

    let mut inst = settings::las(&dir).unwrap_or_default();
    let mut stems = false;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--stems" {
            stems = true;
            i += 1;
        } else if args[i] == "--niva" && i + 1 < args.len() {
            if let Ok(n) = args[i + 1].parse::<u8>() {
                inst = settings::Settings::preset(n);
                inst.niva = n;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    let _ = settings::spara(&dir, &inst);

    print_summary(&song, &file, &inst);

    let mut track = Track::new(SR, song.duration + 2.0);
    arrange::render(&song, &mut track, &inst);
    apply_outro_fade(&mut track, song.duration);
    track.loudness(-13.0, -1.0);

    let out_path = out.unwrap_or_else(|| {
        let name = format!("{}.wav", song.title);
        dir.join("ljud").join(name)
    });
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("kunde inte skapa {}: {e}", parent.display()))?;
        }
    }
    wav::write_wav(&out_path, &track)
        .map_err(|e| format!("kunde inte skriva {}: {e}", out_path.display()))?;

    println!();
    println!("  skrev {}", out_path.display());
    println!(
        "  {} sek, {} frames, {} Hz, stereo 16-bit",
        track.frames / SR as usize,
        track.frames,
        SR as u32
    );
    println!(
        "  rms {:.1} dBFS, topp {:.1} dBFS",
        track.rms_db(),
        track.peak_db()
    );

    if stems {
        let sd = dir.join("ljud").join("stems");
        let filer = arrange::exportera_stems(&song, &inst, &sd)?;
        println!("  {} stems -> {}", filer.len(), sd.display());
        for f in filer {
            println!("    {f}");
        }
    }
    Ok(())
}

/// Skriver en kort sammanfattning av vad som lästes in.
fn print_summary(song: &model::Song, file: &Path, inst: &settings::Settings) {
    println!("basswerk 0.1");
    println!("  fil:      {}", file.display());
    println!("  niva:     {}", inst.niva_namn());
    println!("  titel:    {}", song.title);
    println!("  genre:    {}", song.genre);
    println!("  tempo:    {} BPM  ({:.3} sek/takt)", song.bpm, song.sec_per_bar);
    let k = chords::NOTES[song.key_root as usize];
    let qual = if song.key_minor { "moll" } else { "dur" };
    println!("  tonart:   {k} {qual}");
    println!("  langd:    {} takter, {:.0} sek", song.total_bars, song.duration);
    println!("  sektioner:");
    for s in &song.sections {
        println!(
            "    {:<14} {:>3} takter  {:>6.1}s  {} ackord  [{}]",
            s.name,
            s.bars,
            s.start_sec,
            s.chords.len(),
            s.tags.join(" ")
        );
    }
}

/// Tonar ut sista sekunden.
pub(crate) fn apply_outro_fade(track: &mut Track, end_sec: f64) {
    let fade = 1.2;
    let start = track.frame_at((end_sec - fade).max(0.0));
    let stop = track.frame_at(end_sec);
    for f in start..stop.min(track.frames) {
        let p = (f - start) as f32 / (stop - start).max(1) as f32;
        let g = 1.0 - p;
        track.data[f * 2] *= g;
        track.data[f * 2 + 1] *= g;
    }
    for f in stop..track.frames {
        track.data[f * 2] = 0.0;
        track.data[f * 2 + 1] = 0.0;
    }
}
