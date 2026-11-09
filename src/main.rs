#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const EMBEDDED_MKVMERGE: &[u8] = include_bytes!("../build/win/mkvmerge.exe");

#[derive(Clone, Debug)]
struct EpisodePair {
    enabled: bool,
    ep_key: String,
    hq_path: PathBuf,
    lang_path: PathBuf,
    delay_ms: i32,
    status: String,
}

#[derive(Clone, Debug)]
struct TrackConfig {
    hq_video: bool,
    hq_audio: bool,
    hq_audio_langs: String,
    hq_subs: bool,
    hq_sub_langs: String,
    hq_chapters: bool,
    hq_attachments: bool,

    lang_video: bool,
    lang_audio: bool,
    lang_audio_langs: String,
    lang_audio_default: bool,
    lang_subs: bool,
    lang_sub_langs: String,
    lang_subs_default: bool,
    lang_chapters: bool,
    lang_attachments: bool,
}

impl Default for TrackConfig {
    fn default() -> Self {
        Self {
            hq_video: true,
            hq_audio: true,
            hq_audio_langs: "jpn".to_string(),
            hq_subs: false,
            hq_sub_langs: String::new(),
            hq_chapters: true,
            hq_attachments: true,

            lang_video: false,
            lang_audio: true,
            lang_audio_langs: String::new(),
            lang_audio_default: true,
            lang_subs: true,
            lang_sub_langs: String::new(),
            lang_subs_default: true,
            lang_chapters: false,
            lang_attachments: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OutputNamingMode {
    HqName,
    LangName,
    CustomName,
}

enum WorkerMsg {
    EpisodeStatus(usize, String),
    Log(String),
    Finished,
}

struct AniMergerApp {
    mkvmerge_bin: String,
    hq_dir: String,
    lang_dir: String,
    out_dir: String,
    global_delay_ms: i32,
    naming_mode: OutputNamingMode,
    custom_output_name: String,
    cfg: TrackConfig,
    pairs: Vec<EpisodePair>,
    logs: Vec<String>,
    is_running: bool,
    rx: Option<Receiver<WorkerMsg>>,
}

fn resolve_bundled_mkvmerge() -> String {
    #[cfg(target_os = "windows")]
    {
        let target = std::env::temp_dir().join("animerger_embedded_mkvmerge.exe");
        let needs_write = match fs::metadata(&target) {
            Ok(meta) => meta.len() != EMBEDDED_MKVMERGE.len() as u64,
            Err(_) => true,
        };
        if needs_write {
            let _ = fs::write(&target, EMBEDDED_MKVMERGE);
        }
        return target.to_string_lossy().to_string();
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(appdir) = std::env::var("APPDIR") {
            let bundled = Path::new(&appdir).join("usr/bin/mkvmerge");
            if bundled.exists() {
                return bundled.to_string_lossy().to_string();
            }
        }
        "mkvmerge".to_string()
    }
}

impl Default for AniMergerApp {
    fn default() -> Self {
        let bin = resolve_bundled_mkvmerge();
        Self {
            mkvmerge_bin: bin.clone(),
            hq_dir: String::new(),
            lang_dir: String::new(),
            out_dir: String::new(),
            global_delay_ms: 0,
            naming_mode: OutputNamingMode::HqName,
            custom_output_name: "MyAnime".to_string(),
            cfg: TrackConfig::default(),
            pairs: Vec::new(),
            logs: vec![format!("Ready. Using mkvmerge: {}", bin)],
            is_running: false,
            rx: None,
        }
    }
}

impl AniMergerApp {
    fn is_video_file(path: &Path) -> bool {
        matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase())
                .as_deref(),
            Some("mkv" | "mp4" | "webm" | "avi" | "m2ts")
        )
    }

    fn extract_episode_number(filename: &str) -> Option<u32> {
        let stem = Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename);

        let noise_re = Regex::new(
            r"(?i)(\[[0-9A-F]{8}\]|\b(2160p|1080p|720p|480p|x264|x265|h264|h265|hevc|av1|10bit|8bit|aac|flac|eac3|ac3|opus|1920x1080|1280x720|5\.1|2\.0)\b)",
        )
        .unwrap();
        let cleaned = noise_re.replace_all(stem, " ");

        let se_re = Regex::new(r"(?i)\b(?:S\d{1,2}\s*E|\d{1,2}x)(\d{1,4})(?:v\d)?\b").unwrap();
        if let Some(cap) = se_re.captures(&cleaned) {
            if let Ok(ep) = cap[1].parse::<u32>() {
                return Some(ep);
            }
        }

        let ep_re =
            Regex::new(r"(?i)(?:-\s*|\bep(?:isode)?\.?\s*|\bE)(\d{1,4})(?:v\d)?\b").unwrap();
        if let Some(cap) = ep_re.captures(&cleaned) {
            if let Ok(ep) = cap[1].parse::<u32>() {
                return Some(ep);
            }
        }

        let season_re =
            Regex::new(r"(?i)\b(?:(?:season|saison|s)\s*\d{1,2}|\d{1,2}(?:st|nd|rd|th)\s+season)\b")
                .unwrap();
        let without_season = season_re.replace_all(&cleaned, " ");

        let num_re = Regex::new(r"\b(\d{1,4})(?:v\d)?\b").unwrap();
        num_re
            .captures_iter(&without_season)
            .last()
            .and_then(|cap| cap[1].parse::<u32>().ok())
    }

    fn scan_directory(dir: &str) -> BTreeMap<u32, PathBuf> {
        let mut map = BTreeMap::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && Self::is_video_file(&path) {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if let Some(ep_num) = Self::extract_episode_number(name) {
                            map.insert(ep_num, path);
                        }
                    }
                }
            }
        }
        map
    }

    fn match_folders(&mut self) {
        self.pairs.clear();
        if self.hq_dir.is_empty() || self.lang_dir.is_empty() {
            return;
        }
        let hq_map = Self::scan_directory(&self.hq_dir);
        let lang_map = Self::scan_directory(&self.lang_dir);

        for (ep_num, hq_path) in hq_map {
            if let Some(lang_path) = lang_map.get(&ep_num) {
                self.pairs.push(EpisodePair {
                    enabled: true,
                    ep_key: format!("E{:02}", ep_num),
                    hq_path,
                    lang_path: lang_path.clone(),
                    delay_ms: self.global_delay_ms,
                    status: "Queued".to_string(),
                });
            }
        }
        self.logs.push(format!(
            "Matched {} episode(s) by episode number.",
            self.pairs.len()
        ));
    }

    fn apply_global_delay(&mut self) {
        for pair in &mut self.pairs {
            pair.delay_ms = self.global_delay_ms;
        }
    }

    fn build_mkvmerge_args(
        cfg: &TrackConfig,
        pair: &EpisodePair,
        out_dir: &str,
        mode: OutputNamingMode,
        custom_name: &str,
    ) -> (PathBuf, Vec<String>) {
        let hq_stem = pair
            .hq_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&pair.ep_key);

        let lang_stem = pair
            .lang_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&pair.ep_key);

        let final_filename = match mode {
            OutputNamingMode::HqName => hq_stem.to_string(),
            OutputNamingMode::LangName => lang_stem.to_string(),
            OutputNamingMode::CustomName => {
                let clean = custom_name.trim();
                if clean.is_empty() {
                    pair.ep_key.clone()
                } else {
                    format!("{}_{}", clean, pair.ep_key)
                }
            }
        };

        let out_file = Path::new(out_dir).join(format!("{}.mkv", final_filename));

        let mut args = vec!["-o".to_string(), out_file.to_string_lossy().to_string()];

        if !cfg.hq_video {
            args.push("--no-video".into());
        }
        if !cfg.hq_audio {
            args.push("--no-audio".into());
        } else if !cfg.hq_audio_langs.trim().is_empty() {
            args.push("--audio-tracks".into());
            args.push(cfg.hq_audio_langs.trim().into());
        }
        if !cfg.hq_subs {
            args.push("--no-subtitles".into());
        } else if !cfg.hq_sub_langs.trim().is_empty() {
            args.push("--subtitle-tracks".into());
            args.push(cfg.hq_sub_langs.trim().into());
        }
        if !cfg.hq_chapters {
            args.push("--no-chapters".into());
        }
        if !cfg.hq_attachments {
            args.push("--no-attachments".into());
        }
        args.push(pair.hq_path.to_string_lossy().to_string());

        if !cfg.lang_video {
            args.push("--no-video".into());
        }
        if !cfg.lang_audio {
            args.push("--no-audio".into());
        } else {
            if !cfg.lang_audio_langs.trim().is_empty() {
                args.push("--audio-tracks".into());
                args.push(cfg.lang_audio_langs.trim().into());
            }
            if cfg.lang_audio_default {
                args.push("--default-track-flag".into());
                args.push("0:yes".into());
            }
        }
        if !cfg.lang_subs {
            args.push("--no-subtitles".into());
        } else {
            if !cfg.lang_sub_langs.trim().is_empty() {
                args.push("--subtitle-tracks".into());
                args.push(cfg.lang_sub_langs.trim().into());
            }
            if cfg.lang_subs_default {
                args.push("--default-track-flag".into());
                args.push("0:yes".into());
            }
        }
        if !cfg.lang_chapters {
            args.push("--no-chapters".into());
        }
        if !cfg.lang_attachments {
            args.push("--no-attachments".into());
        }
        if pair.delay_ms != 0 {
            args.push("--sync".into());
            args.push(format!("-1:{}", pair.delay_ms));
        }
        args.push(pair.lang_path.to_string_lossy().to_string());

        (out_file, args)
    }

    fn start_merge(&mut self) {
        if self.is_running || self.out_dir.trim().is_empty() {
            self.logs
                .push("Error: Please select an output folder before starting.".into());
            return;
        }
        let _ = fs::create_dir_all(&self.out_dir);

        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.is_running = true;

        let bin = self.mkvmerge_bin.clone();
        let cfg = self.cfg.clone();
        let out_dir = self.out_dir.clone();
        let mode = self.naming_mode;
        let custom_name = self.custom_output_name.clone();
        let jobs: Vec<(usize, EpisodePair)> = self
            .pairs
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, p)| p.enabled)
            .collect();

        thread::spawn(move || {
            for (idx, pair) in jobs {
                let _ = tx.send(WorkerMsg::EpisodeStatus(idx, "Muxing...".into()));
                let (out_file, args) = Self::build_mkvmerge_args(&cfg, &pair, &out_dir, mode, &custom_name);

                let _ = tx.send(WorkerMsg::Log(format!(
                    "[{}] Running: {} {}",
                    pair.ep_key,
                    bin,
                    args.join(" ")
                )));

                let mut cmd = Command::new(&bin);
                cmd.args(&args);

                #[cfg(target_os = "windows")]
                cmd.creation_flags(0x08000000);

                match cmd.output() {
                    Ok(output) => {
                        let code = output.status.code().unwrap_or(2);
                        if code == 0 || code == 1 {
                            let status_label = if code == 0 { "Done" } else { "Done (Warn)" };
                            let _ = tx.send(WorkerMsg::EpisodeStatus(idx, status_label.into()));
                            let _ = tx.send(WorkerMsg::Log(format!(
                                "[{}] Saved -> {}",
                                pair.ep_key,
                                out_file.display()
                            )));
                        } else {
                            let err = String::from_utf8_lossy(&output.stdout);
                            let _ = tx.send(WorkerMsg::EpisodeStatus(idx, "Failed".into()));
                            let _ = tx.send(WorkerMsg::Log(format!(
                                "[{}] mkvmerge error: {}",
                                pair.ep_key, err
                            )));
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(WorkerMsg::EpisodeStatus(idx, "Error".into()));
                        let _ = tx.send(WorkerMsg::Log(format!(
                            "Failed to execute '{}': {}",
                            bin, e
                        )));
                    }
                }
            }
            let _ = tx.send(WorkerMsg::Finished);
        });
    }
}

impl eframe::App for AniMergerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    WorkerMsg::EpisodeStatus(idx, status) => {
                        if let Some(p) = self.pairs.get_mut(idx) {
                            p.status = status;
                        }
                    }
                    WorkerMsg::Log(line) => self.logs.push(line),
                    WorkerMsg::Finished => {
                        self.is_running = false;
                        self.logs.push("Batch merge finished.".into());
                    }
                }
            }
            if self.is_running {
                ctx.request_repaint();
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("AniMerger — High Quality Video + Target Language Muxer");
            ui.add_space(6.0);

            egui::Grid::new("paths_grid")
                .num_columns(3)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label("High Quality Folder:");
                    ui.add(egui::TextEdit::singleline(&mut self.hq_dir).desired_width(520.0));
                    if ui.button("Browse...").clicked() {
                        if let Some(p) = rfd::FileDialog::new().pick_folder() {
                            self.hq_dir = p.display().to_string();
                            self.match_folders();
                        }
                    }
                    ui.end_row();

                    ui.label("Right Language Folder:");
                    ui.add(egui::TextEdit::singleline(&mut self.lang_dir).desired_width(520.0));
                    if ui.button("Browse...").clicked() {
                        if let Some(p) = rfd::FileDialog::new().pick_folder() {
                            self.lang_dir = p.display().to_string();
                            self.match_folders();
                        }
                    }
                    ui.end_row();

                    ui.label("Output Folder:");
                    ui.add(egui::TextEdit::singleline(&mut self.out_dir).desired_width(520.0));
                    if ui.button("Browse...").clicked() {
                        if let Some(p) = rfd::FileDialog::new().pick_folder() {
                            self.out_dir = p.display().to_string();
                        }
                    }
                    ui.end_row();

                    ui.label("Output File Name:");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.naming_mode, OutputNamingMode::HqName, "HQ File Name");
                        ui.radio_value(&mut self.naming_mode, OutputNamingMode::LangName, "Right Language File Name");
                        ui.radio_value(&mut self.naming_mode, OutputNamingMode::CustomName, "Custom Name");

                        if self.naming_mode == OutputNamingMode::CustomName {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.custom_output_name)
                                    .desired_width(180.0)
                                    .hint_text("MyAnime"),
                            );
                            ui.label("(_E01)");
                        }
                    });
                    ui.end_row();

                    ui.label("Global Sync Delay (ms):");
                    let response = ui.add(
                        egui::DragValue::new(&mut self.global_delay_ms)
                            .speed(50)
                            .suffix(" ms"),
                    );
                    if response.changed() {
                        self.apply_global_delay();
                    }
                    ui.end_row();

                    ui.label("mkvmerge Binary:");
                    ui.add(egui::TextEdit::singleline(&mut self.mkvmerge_bin).desired_width(520.0));
                    if ui.button("Rescan Folders").clicked() {
                        self.match_folders();
                    }
                    ui.end_row();
                });

            ui.separator();

            ui.columns(2, |cols| {
                cols[0].group(|ui| {
                    ui.strong("Keep from High Quality Folder (Source A)");
                    ui.checkbox(&mut self.cfg.hq_video, "Video Tracks (Best Video)");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.cfg.hq_audio, "Audio Tracks — Lang filter:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cfg.hq_audio_langs)
                                .hint_text("e.g. jpn (blank = all)")
                                .desired_width(110.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.cfg.hq_subs, "Subtitles — Lang filter:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cfg.hq_sub_langs)
                                .hint_text("e.g. eng (blank = all)")
                                .desired_width(110.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.cfg.hq_chapters, "Chapters");
                        ui.checkbox(&mut self.cfg.hq_attachments, "Fonts / Attachments");
                    });
                });

                cols[1].group(|ui| {
                    ui.strong("Keep from Right Language Folder (Source B)");
                    ui.checkbox(&mut self.cfg.lang_video, "Video Tracks");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.cfg.lang_audio, "Audio Tracks — Lang filter:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cfg.lang_audio_langs)
                                .hint_text("e.g. fra,eng (blank = all)")
                                .desired_width(110.0),
                        );
                        ui.checkbox(&mut self.cfg.lang_audio_default, "Default");
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.cfg.lang_subs, "Subtitles — Lang filter:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cfg.lang_sub_langs)
                                .hint_text("e.g. fra,eng (blank = all)")
                                .desired_width(110.0),
                        );
                        ui.checkbox(&mut self.cfg.lang_subs_default, "Default");
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.cfg.lang_chapters, "Chapters");
                        ui.checkbox(&mut self.cfg.lang_attachments, "Fonts / Attachments");
                    });
                });
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.strong(format!("Matched Episodes ({})", self.pairs.len()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            !self.is_running && !self.pairs.is_empty(),
                            egui::Button::new("Start Merge Batch"),
                        )
                        .clicked()
                    {
                        self.start_merge();
                    }
                });
            });

            egui::ScrollArea::vertical()
                .id_source("pairs_scroll")
                .max_height(230.0)
                .show(ui, |ui| {
                    egui::Grid::new("pairs_grid")
                        .striped(true)
                        .num_columns(6)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.strong("Use");
                            ui.strong("Ep");
                            ui.strong("High Quality File (Source A)");
                            ui.strong("Language File (Source B)");
                            ui.strong("Sync (ms)");
                            ui.strong("Status");
                            ui.end_row();

                            for pair in &mut self.pairs {
                                ui.checkbox(&mut pair.enabled, "");
                                ui.label(&pair.ep_key);
                                ui.label(
                                    pair.hq_path
                                        .file_name()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or(""),
                                );
                                ui.label(
                                    pair.lang_path
                                        .file_name()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or(""),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut pair.delay_ms)
                                        .speed(50)
                                        .suffix(" ms"),
                                );
                                ui.label(&pair.status);
                                ui.end_row();
                            }
                        });
                });

            ui.separator();

            ui.strong("mkvmerge Output Log");
            egui::ScrollArea::vertical()
                .id_source("logs_scroll")
                .stick_to_bottom(true)
                .max_height(120.0)
                .show(ui, |ui| {
                    for line in &self.logs {
                        ui.monospace(line);
                    }
                });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 700.0])
            .with_min_inner_size([850.0, 580.0]),
        ..Default::default()
    };
    eframe::run_native(
        "AniMerger",
        options,
        Box::new(|_cc| Ok(Box::new(AniMergerApp::default()))),
    )
}