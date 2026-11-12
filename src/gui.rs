//! Fönstergränssnitt: välj mapp, generera, få musik.
use crate::{arrange, load, wav};
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

/// Sammanfattning av den valda låten.
#[derive(Default)]
pub struct Info {
    pub fil: String,
    pub titel: String,
    pub genre: String,
    pub tempo: String,
    pub tonart: String,
    pub langd: String,
    pub sektioner: Vec<String>,
}

/// Startar fönstret.
pub fn run(mapp: Option<PathBuf>) -> Result<(), String> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([580.0, 640.0])
            .with_min_inner_size([420.0, 480.0])
            .with_title("basswerk"),
        ..Default::default()
    };
    eframe::run_native(
        "basswerk",
        opts,
        Box::new(move |_cc| Ok(Box::new(App::new(mapp)))),
    )
    .map_err(|e| format!("kunde inte starta fönstret: {e}"))
}

pub fn granska(dir: &Path) -> Result<Info, String> {
    let file = load::find_song_file(dir)
        .ok_or_else(|| format!("hittade ingen .txt i {}", dir.display()))?;
    let song = load::load(&file)?;
    let qual = if song.key_minor { "moll" } else { "dur" };
    Ok(Info {
        fil: file.file_name().unwrap_or_default().to_string_lossy().to_string(),
        titel: song.title.clone(),
        genre: song.genre.clone(),
        tempo: format!("{:.0} BPM  ({:.3} sek/takt)", song.bpm, song.sec_per_bar),
        tonart: format!("{} {}", crate::chords::NOTES[song.key_root as usize], qual),
        langd: format!("{} takter, {:.0} sek", song.total_bars, song.duration),
        sektioner: song
            .sections
            .iter()
            .map(|s| format!("{:<14} {:>3} takter  {:>6.1}s", s.name, s.bars, s.start_sec))
            .collect(),
    })
}

/// Meddelanden från en bakgrundskörning.
pub enum Jobb {
    Framsteg(String),
    Klar(String),
    Fel(String),
}

pub struct App {
    mapp: Option<PathBuf>,
    info: Option<Info>,
    status: String,
    busy: bool,
    rx: Option<Receiver<Jobb>>,
    senaste: Option<PathBuf>,
    inst: crate::settings::Settings,
    sink: Option<rodio::MixerDeviceSink>,
    spelare: Option<rodio::Player>,
    amne: String,
    skapar: bool,
    app: crate::settings::AppSettings,
}

impl App {
    pub fn new(mapp: Option<PathBuf>) -> App {
        let mut a = App {
            mapp: None,
            info: None,
            status: "Välj en låtmapp, eller släpp in den i fönstret.".into(),
            busy: false,
            rx: None,
            senaste: None,
            inst: crate::settings::Settings::default(),
            sink: None,
            spelare: None,
            amne: String::new(),
            skapar: false,
            app: crate::settings::las_app(),
        };
        if let Some(d) = mapp {
            a.valj(&d);
        }
        a
    }

    pub fn valj(&mut self, dir: &Path) {
        self.mapp = Some(dir.to_path_buf());
        self.senaste = None;
        self.inst = crate::settings::las(dir).unwrap_or_default();
        match granska(dir) {
            Ok(i) => {
                self.status = format!("Redo: {}", i.titel);
                self.info = Some(i);
            }
            Err(e) => {
                self.status = e;
                self.info = None;
            }
        }
    }

    /// Startar uppspelning av en WAV i appen.
    fn spela(&mut self, path: &Path) {
        self.spelare = None;
        if self.sink.is_none() {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(s) => self.sink = Some(s),
                Err(e) => {
                    self.status = format!("ingen ljudenhet: {e}");
                    return;
                }
            }
        }
        let fil = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                self.status = format!("kunde inte öppna: {e}");
                return;
            }
        };
        let res = {
            let sink = self.sink.as_ref().expect("sink finns");
            rodio::play(sink.mixer(), std::io::BufReader::new(fil))
        };
        match res {
            Ok(p) => {
                p.set_volume(0.9);
                self.spelare = Some(p);
                self.status = "Spelar...".into();
            }
            Err(e) => self.status = format!("kunde inte spela: {e}"),
        }
    }

    /// Startar hela kedjan: text via opencode, sedan musik.
    fn skapa_lat(&mut self) {
        if self.busy || self.amne.trim().is_empty() {
            return;
        }
        self.busy = true;
        self.skapar = true;
        self.senaste = None;
        self.spelare = None;
        self.status = "Skriver text... (1-4 min)".into();
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let amne = self.amne.trim().to_string();
        let niva = self.app.textniva;
        let genre = self.app.genre.clone();
        let app = self.app.clone();
        std::thread::spawn(move || {
            match skapa_jobb(&app, &amne, niva, &genre) {
                Ok(p) => {
                    let _ = tx.send(Jobb::Klar(p));
                }
                Err(e) => {
                    let _ = tx.send(Jobb::Fel(e));
                }
            }
        });    }

    /// Genererar om alla låtar under arbetsmappens `låtar\`.
    fn generera_alla(&mut self) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.skapar = false;
        self.spelare = None;
        self.status = "Genererar alla...".into();
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let arbete = self.app.arbetsmapp.clone();
        std::thread::spawn(move || {
            let latar = PathBuf::from(&arbete).join("låtar");
            let alla = mappar(&latar);
            let n = alla.len();
            if n == 0 {
                let _ = tx.send(Jobb::Fel("hittade inga låtar".into()));
                return;
            }
            for (i, m) in alla.iter().enumerate() {
                let namn = m
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let _ = tx.send(Jobb::Framsteg(format!("{}/{}  {namn}", i + 1, n)));
                let inst = crate::settings::las(m).unwrap_or_default();
                if let Err(e) = jobb(m, inst) {
                    let _ = tx.send(Jobb::Fel(format!("{namn}: {e}")));
                    return;
                }
            }
            let _ = tx.send(Jobb::Klar(format!("{n} låtar klara")));
        });
    }

    fn generera(&mut self) {
        let Some(dir) = self.mapp.clone() else { return };
        if self.busy {
            return;
        }
        self.busy = true;
        self.skapar = false;
        self.senaste = None;
        self.spelare = None;
        self.status = "Genererar musiken...".into();
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let inst = self.inst.clone();
        std::thread::spawn(move || {
            match jobb(&dir, inst) {
                Ok(p) => {
                    let _ = tx.send(Jobb::Klar(p));
                }
                Err(e) => {
                    let _ = tx.send(Jobb::Fel(e));
                }
            }
        });
    }
}

/// Första WAV-filen i `<mapp>\ljud\`, om det finns någon.
fn nyaste_wav(dir: &Path) -> Option<PathBuf> {
    let ljud = dir.join("ljud");
    std::fs::read_dir(&ljud)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().map(|x| x.eq_ignore_ascii_case("wav")) == Some(true))
}

/// Letar upp opencode: först inställningen, sedan %APPDATA%\npm och PATH.
fn hitta_opencode(app: &crate::settings::AppSettings) -> Option<PathBuf> {
    if !app.opencode_sokvag.trim().is_empty() {
        let p = PathBuf::from(app.opencode_sokvag.trim());
        if p.is_file() {
            return Some(p);
        }
        return None;
    }
    let namn = ["opencode.cmd", "opencode.exe", "opencode.bat"];
    if let Ok(appdata) = std::env::var("APPDATA") {
        for n in namn {
            let p = PathBuf::from(&appdata).join("npm").join(n);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            for n in namn {
                let p = dir.join(n);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// Alla låtmappar under `låtar\` (två nivåer ner).
fn mappar(latar: &Path) -> Vec<PathBuf> {
    let mut ut = Vec::new();
    if let Ok(niva) = std::fs::read_dir(latar) {
        for n in niva.flatten() {
            if let Ok(s) = std::fs::read_dir(n.path()) {
                for d in s.flatten() {
                    if d.path().is_dir() {
                        ut.push(d.path());
                    }
                }
            }
        }
    }
    ut
}

/// Kör hela kedjan: opencode skriver texten, vi hittar mappen och gör musiken.
fn skapa_jobb(
    app: &crate::settings::AppSettings,
    amne: &str,
    niva: u8,
    genre: &str,
) -> Result<String, String> {
    let arbete = PathBuf::from(&app.arbetsmapp);
    if !arbete.is_dir() {
        return Err(format!("arbetsmappen finns inte: {}", arbete.display()));
    }
    let latar = arbete.join("låtar");
    let fore = mappar(&latar);

    let prompt = format!(
        "@latstruktur-mall Skriv en låt om \"{amne}\". Genre: {genre}. Explicit-nivå {niva}. \
         Fråga inte om något - kör bara. Leverera mappen enligt skillen. \
         Kör INTE basswerk, musiken genereras separat."
    );
    let oc = hitta_opencode(app).ok_or(
        "hittade inte opencode. Letade i %APPDATA%\\npm och PATH - ange sokvag i installning.txt",
    )?;
    let ut = std::process::Command::new("cmd.exe")
        .arg("/C")
        .arg(&oc)
        .args(["run", "-m", &app.modell, &prompt])
        .current_dir(&arbete)
        .output()
        .map_err(|e| format!("kunde inte starta opencode: {e}"))?;
    if !ut.status.success() {
        let fel = String::from_utf8_lossy(&ut.stderr);
        let kort: String = fel.chars().take(160).collect();
        return Err(format!("opencode misslyckades: {kort}"));
    }

    let efter = mappar(&latar);
    let ny = efter
        .iter()
        .find(|p| !fore.contains(p))
        .cloned()
        .ok_or("hittade ingen ny låtmapp")?;

    let exe = std::env::current_exe().map_err(|e| format!("{e}"))?;
    let ut2 = std::process::Command::new(exe)
        .arg(&ny)
        .args(["--niva", "5"])
        .output()
        .map_err(|e| format!("kunde inte starta musiken: {e}"))?;
    if !ut2.status.success() {
        return Err("texten blev klar men musiken misslyckades".into());
    }
    Ok(ny.display().to_string())
}

/// Kör hela kedjan och skriver WAV i `<mapp>\ljud\`.
fn jobb(dir: &Path, inst: crate::settings::Settings) -> Result<String, String> {
    let file = load::find_song_file(dir).ok_or("hittade ingen .txt i mappen")?;
    let song = load::load(&file)?;
    let mut track = wav::Track::new(crate::SR, song.duration + 2.0);
    arrange::render(&song, &mut track, &inst);
    let _ = crate::settings::spara(dir, &inst);
    crate::apply_outro_fade(&mut track, song.duration);
    track.loudness(-13.0, -1.0);

    let ljud = dir.join("ljud");
    std::fs::create_dir_all(&ljud).map_err(|e| format!("kunde inte skapa ljud: {e}"))?;
    let out = ljud.join(format!("{}.wav", song.title));
    wav::write_wav(&out, &track).map_err(|e| format!("kunde inte skriva filen: {e}"))?;

    if inst.stems {
        let sd = dir.join("ljud").join("stems");
        let filer = arrange::exportera_stems(&song, &inst, &sd)?;
        return Ok(format!("{}  (+{} stems i stems\\)", out.display(), filer.len()));
    }
    Ok(out.display().to_string())
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _f: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(Jobb::Framsteg(s)) => self.status = s,
                Ok(Jobb::Klar(p)) => {
                    self.busy = false;
                    self.rx = None;
                    let skapade = self.skapar;
                    self.skapar = false;
                    if skapade {
                        let d = PathBuf::from(&p);
                        self.valj(&d);
                        self.senaste = nyaste_wav(&d);
                        self.status = format!("Ny låt klar: {p}");
                    } else {
                        self.status = format!("Klar: {p}");
                        self.senaste = Some(PathBuf::from(p));
                    }
                }
                Ok(Jobb::Fel(e)) => {
                    self.busy = false;
                    self.rx = None;
                    self.skapar = false;
                    self.status = format!("Fel: {e}");
                }
                Err(_) => ctx.request_repaint(),
            }
        }

        if self.spelare.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }

        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(f) = dropped.first() {
            {
                let p = f.path();
                let dir = if p.is_dir() {
                    p.to_path_buf()
                } else {
                    p.parent()
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| p.to_path_buf())
                };
                self.valj(&dir);
            }
        }

        let st = ui.style().clone();
        egui::Frame::central_panel(&st).show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| self.rita(ui));
        });
    }
}

impl App {
    fn rita(&mut self, ui: &mut egui::Ui) {
        ui.heading("basswerk");
        ui.add_space(4.0);
        ui.strong("SKAPA NY LÅT");
        ui.weak("opencode skriver texten, basswerk gör musiken. Tar 1-4 minuter.");
        ui.add_space(4.0);
        ui.label("Vad ska låten handla om?");
        ui.add_enabled(
            !self.busy,
            egui::TextEdit::singleline(&mut self.amne).desired_width(430.0),
        );
        ui.horizontal(|ui| {
            ui.label("Textnivå");
            for n in 1..=5u8 {
                let vald = self.app.textniva == n;
                if ui.selectable_label(vald, format!("{n}")).clicked() {
                    self.app.textniva = n;
                    let _ = crate::settings::spara_app(&self.app);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Genre    ");
            let mut vald = self.app.genre.clone();
            egui::ComboBox::from_id_salt("genre")
                .selected_text(vald.clone())
                .show_ui(ui, |ui| {
                    for g in crate::settings::GENRER {
                        ui.selectable_value(&mut vald, g.to_string(), g);
                    }
                });
            if vald != self.app.genre {
                self.app.genre = vald;
                let _ = crate::settings::spara_app(&self.app);
            }
        });
        let kan = !self.busy && !self.amne.trim().is_empty();
        if ui.add_enabled(kan, egui::Button::new("Skapa låt")).clicked() {
            self.skapa_lat();
        }
        ui.weak("Textnivå 1 = flörtig, 3 = rak, 5 = max. Genren styr textens stil.");
        let niva_mapp = match self.app.textniva {
            1 | 2 => "nivå1-2",
            3 | 4 => "nivå3-4",
            _ => "nivå5",
        };
        let bas = if self.app.arbetsmapp.trim().is_empty() {
            "<arbetsmapp>"
        } else {
            self.app.arbetsmapp.as_str()
        };
        ui.weak(format!("Hamnar i: {bas}\\låtar\\{niva_mapp}\\<titel>"));
        ui.separator();
        ui.add_space(4.0);
        ui.strong("BEFINTLIG LÅT");
        ui.weak("Öppna en låt som redan finns, och gör om musiken med inställningarna nedan.");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("Öppna mapp...").clicked() {
                if let Some(d) = rfd::FileDialog::new()
                    .set_title("Välj låtmapp")
                    .pick_folder()
                {
                    self.valj(&d);
                }
            }
            if self.busy {
                ui.spinner();
            }
        });
        ui.weak("Välj en mapp som redan har låttexter. Musiken görs med inställningarna nedan.");

        ui.add_space(6.0);
        ui.weak("Ny låt? Skriv  /låt <ämne>  i OpenCode - den skapar mappen. Välj den sedan här.");
        ui.add_space(2.0);
        match &self.mapp {
            Some(d) => ui.label(format!("Mapp: {}", d.display())),
            None => ui.weak("Ingen mapp vald. Du kan också släppa in en mapp i fönstret."),
        };

        ui.separator();
        ui.label(format!("Produktionsnivå:  {}", self.inst.niva_namn()));
        ui.horizontal(|ui| {
            for n in 1..=5u8 {
                let vald = self.inst.niva == n;
                if ui.selectable_label(vald, format!("{n}")).clicked() {
                    self.inst = crate::settings::Settings::preset(n);
                }
            }
        });
        ui.add_space(4.0);
        egui::Grid::new("element")
            .num_columns(3)
            .spacing([18.0, 4.0])
            .show(ui, |ui| {
                for (i, e) in crate::settings::ALLA.iter().enumerate() {
                    let mut v = self.inst.get(*e);
                    if ui.checkbox(&mut v, e.namn()).changed() {
                        self.inst.set(*e, v);
                    }
                    if (i + 1) % 3 == 0 {
                        ui.end_row();
                    }
                }
            });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label("Effektmängd");
            let mut x = self.inst.fx_mangd;
            if ui
                .add(egui::Slider::new(&mut x, 0.0..=1.5).fixed_decimals(2))
                .changed()
            {
                self.inst.fx_mangd = x;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Lead-nivå    ");
            let mut x = self.inst.lead_mangd;
            if ui
                .add(egui::Slider::new(&mut x, 0.0..=1.0).fixed_decimals(2))
                .changed()
            {
                self.inst.lead_mangd = x;
            }
        });
        ui.checkbox(&mut self.inst.stems, "Exportera stems (en WAV per buss)");

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Ackordfärg");
            for (n, namn) in ["Ren", "7:a", "+9", "Sus"].iter().enumerate() {
                let vald = self.inst.farg as usize == n;
                if ui.selectable_label(vald, *namn).clicked() {
                    self.inst.farg = n as u8;
                }
            }
        });
        ui.weak("Ren = treklang som i texten. 7:a, +9 och Sus färgar ackorden.");

        ui.add_space(4.0);
        ui.label("Volym per element");
        egui::Grid::new("bussar")
            .num_columns(4)
            .spacing([12.0, 2.0])
            .show(ui, |ui| {
                for (i, n) in crate::settings::buss_namn().iter().enumerate() {
                    ui.label(*n);
                    ui.add(
                        egui::Slider::new(&mut self.inst.buss_gain[i], 0.0..=1.5)
                            .fixed_decimals(2),
                    );
                    if i % 2 == 1 {
                        ui.end_row();
                    }
                }
            });

        ui.add_space(6.0);
        let kan = self.mapp.is_some() && !self.busy;
        if ui
            .add_enabled(kan, egui::Button::new("Generera musiken"))
            .clicked()
        {
            self.generera();
        }
        ui.weak("Gör om musiken för mappen ovan med inställningarna du valt.");
        ui.add_space(4.0);
        if ui
            .add_enabled(!self.busy, egui::Button::new("Generera alla låtar"))
            .clicked()
        {
            self.generera_alla();
        }
        ui.weak("Kör om varje låt under låtar\\ med sin egen mix. Några sekunder per låt.");
        ui.add_space(4.0);
        ui.label(&self.status);

        if let Some(p) = self.senaste.clone() {
            ui.horizontal(|ui| {
                let spelar = self.spelare.is_some();
                if ui.add_enabled(!spelar, egui::Button::new("Spela")).clicked() {
                    self.spela(&p);
                }
                if ui.add_enabled(spelar, egui::Button::new("Stopp")).clicked() {
                    self.spelare = None;
                    self.status = "Stoppad.".into();
                }
                if ui.button("Öppna i spelare").clicked() {
                    let _ = std::process::Command::new("cmd")
                        .args(["/C", "start", ""])
                        .arg(&p)
                        .spawn();
                }
            });
        }

        if let Some(i) = &self.info {
            ui.separator();
            egui::Grid::new("info").num_columns(2).striped(true).show(ui, |ui| {
                ui.label("Titel"); ui.label(&i.titel); ui.end_row();
                ui.label("Fil"); ui.label(&i.fil); ui.end_row();
                ui.label("Genre"); ui.label(&i.genre); ui.end_row();
                ui.label("Tempo"); ui.label(&i.tempo); ui.end_row();
                ui.label("Tonart"); ui.label(&i.tonart); ui.end_row();
                ui.label("Längd"); ui.label(&i.langd); ui.end_row();
            });
            ui.add_space(6.0);
            egui::CollapsingHeader::new(format!("Struktur ({} sektioner)", i.sektioner.len()))
                .default_open(true)
                .show(ui, |ui| {
                    for rad in &i.sektioner {
                        ui.monospace(rad);
                    }
                });
        }

        ui.separator();
        egui::CollapsingHeader::new("Inställningar")
            .default_open(false)
            .show(ui, |ui| {
                let mut andrad = false;
                ui.horizontal(|ui| {
                    ui.label("Arbetsmapp");
                    andrad |= ui.text_edit_singleline(&mut self.app.arbetsmapp).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Modell      ");
                    andrad |= ui.text_edit_singleline(&mut self.app.modell).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("opencode  ");
                    andrad |= ui.text_edit_singleline(&mut self.app.opencode_sokvag).changed();
                });
                ui.weak("Sökväg till opencode. Tom = leta själv i %APPDATA%\\npm och PATH.");
                if andrad {
                    let _ = crate::settings::spara_app(&self.app);
                }
                ui.weak(format!(
                    "Sparas i {}",
                    crate::settings::app_sokvag().display()
                ));
            });
    }
}
