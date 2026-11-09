#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use regex::Regex;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const EMBEDDED_MKVMERGE: &[u8] = include_bytes!("../build/win/mkvmerge.exe");

#[cfg(target_os = "windows")]
const EMBEDDED_ALASS: &[u8] = include_bytes!("../build/win/alass.exe");

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
    
    verify_integrity: bool,
    auto_sync_subs: bool,
}

impl Default for TrackConfig {
    fn default() -> Self {
        Self {
            hq_video: true,
            hq_audio: true,
            hq_audio_langs: String::new(),
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

            verify_integrity: true,
            auto_sync_subs: false,
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
    Progress(f32, String),
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
    all_hq_files: Vec<PathBuf>,
    all_lang_files: Vec<PathBuf>,
    selected_manual_hq: Option<PathBuf>,
    selected_manual_lang: Option<PathBuf>,
    manual_ep_key: String,
    logs: Vec<String>,
    is_running: bool,
    progress: f32,
    progress_text: String,
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

    #[cfg(target_os = "macos")]
    {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let bundled = parent.join("mkvmerge");
                if bundled.exists() {
                    return bundled.to_string_lossy().to_string();
                }
            }
        }
        let brew_path = Path::new("/opt/homebrew/bin/mkvmerge");
        if brew_path.exists() {
            return brew_path.to_string_lossy().to_string();
        }
        let intel_brew_path = Path::new("/usr/local/bin/mkvmerge");
        if intel_brew_path.exists() {
            return intel_brew_path.to_string_lossy().to_string();
        }
        "mkvmerge".to_string()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
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

fn resolve_bundled_alass() -> String {
    #[cfg(target_os = "windows")]
    {
        let target = std::env::temp_dir().join("animerger_embedded_alass.exe");
        let needs_write = match fs::metadata(&target) {
            Ok(meta) => meta.len() != EMBEDDED_ALASS.len() as u64,
            Err(_) => true,
        };
        if needs_write {
            let _ = fs::write(&target, EMBEDDED_ALASS);
        }
        return target.to_string_lossy().to_string();
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let bundled = parent.join("alass");
                if bundled.exists() {
                    return bundled.to_string_lossy().to_string();
                }
            }
        }
        let brew_path = Path::new("/opt/homebrew/bin/alass");
        if brew_path.exists() {
            return brew_path.to_string_lossy().to_string();
        }
        let intel_brew_path = Path::new("/usr/local/bin/alass");
        if intel_brew_path.exists() {
            return intel_brew_path.to_string_lossy().to_string();
        }
        "alass".to_string()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Ok(appdir) = std::env::var("APPDIR") {
            let bundled = Path::new(&appdir).join("usr/bin/alass");
            if bundled.exists() {
                return bundled.to_string_lossy().to_string();
            }
        }
        "alass".to_string()
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
            all_hq_files: Vec::new(),
            all_lang_files: Vec::new(),
            selected_manual_hq: None,
            selected_manual_lang: None,
            manual_ep_key: "E01".to_string(),
            logs: vec![format!("Ready. Using mkvmerge: {}", bin)],
            is_running: false,
            progress: 0.0,
            progress_text: "Idle".to_string(),
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

    fn list_video_files(dir: &str) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && Self::is_video_file(&path) {
                    files.push(path);
                }
            }
        }
        files.sort();
        files
    }

    fn scan_directory(dir: &str) -> BTreeMap<u32, PathBuf> {
        let mut map = BTreeMap::new();
        for path in Self::list_video_files(dir) {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(ep_num) = Self::extract_episode_number(name) {
                    map.insert(ep_num, path);
                }
            }
        }
        map
    }

    fn refresh_file_lists(&mut self) {
        self.all_hq_files = if self.hq_dir.is_empty() {
            Vec::new()
        } else {
            Self::list_video_files(&self.hq_dir)
        };

        self.all_lang_files = if self.lang_dir.is_empty() {
            Vec::new()
        } else {
            Self::list_video_files(&self.lang_dir)
        };
    }

    fn update_default_manual_selection(&mut self) {
        let matched_hq: HashSet<_> = self.pairs.iter().map(|p| &p.hq_path).collect();
        let matched_lang: HashSet<_> = self.pairs.iter().map(|p| &p.lang_path).collect();

        if self
            .selected_manual_hq
            .as_ref()
            .map_or(true, |p| !self.all_hq_files.contains(p))
        {
            self.selected_manual_hq = self
                .all_hq_files
                .iter()
                .find(|p| !matched_hq.contains(p))
                .or_else(|| self.all_hq_files.first())
                .cloned();
        }

        if self
            .selected_manual_lang
            .as_ref()
            .map_or(true, |p| !self.all_lang_files.contains(p))
        {
            self.selected_manual_lang = self
                .all_lang_files
                .iter()
                .find(|p| !matched_lang.contains(p))
                .or_else(|| self.all_lang_files.first())
                .cloned();
        }

        self.suggest_manual_ep_key();
    }

    fn suggest_manual_ep_key(&mut self) {
        if let Some(hq) = &self.selected_manual_hq {
            if let Some(name) = hq.file_name().and_then(|s| s.to_str()) {
                if let Some(ep) = Self::extract_episode_number(name) {
                    self.manual_ep_key = format!("E{:02}", ep);
                    return;
                }
            }
        }
        if let Some(lang) = &self.selected_manual_lang {
            if let Some(name) = lang.file_name().and_then(|s| s.to_str()) {
                if let Some(ep) = Self::extract_episode_number(name) {
                    self.manual_ep_key = format!("E{:02}", ep);
                    return;
                }
            }
        }
        self.manual_ep_key = format!("E{:02}", self.pairs.len() + 1);
    }

    fn match_folders(&mut self) {
        self.refresh_file_lists();
        self.pairs.clear();

        if !self.hq_dir.is_empty() && !self.lang_dir.is_empty() {
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
                "Auto-matched {} episode(s) by episode number.",
                self.pairs.len()
            ));
        }

        self.selected_manual_hq = None;
        self.selected_manual_lang = None;
        self.update_default_manual_selection();
    }

    fn add_manual_pair(&mut self) {
        if let (Some(hq_path), Some(lang_path)) = (
            self.selected_manual_hq.clone(),
            self.selected_manual_lang.clone(),
        ) {
            let ep_key = if self.manual_ep_key.trim().is_empty() {
                format!("E{:02}", self.pairs.len() + 1)
            } else {
                self.manual_ep_key.trim().to_string()
            };

            self.logs.push(format!(
                "Manually paired [{}]: {} + {}",
                ep_key,
                hq_path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
                lang_path.file_name().and_then(|s| s.to_str()).unwrap_or("")
            ));

            self.pairs.push(EpisodePair {
                enabled: true,
                ep_key,
                hq_path,
                lang_path,
                delay_ms: self.global_delay_ms,
                status: "Queued (Manual)".to_string(),
            });

            self.selected_manual_hq = None;
            self.selected_manual_lang = None;
            self.update_default_manual_selection();
        }
    }

    fn apply_global_delay(&mut self) {
        for pair in &mut self.pairs {
            pair.delay_ms = self.global_delay_ms;
        }
    }

    fn build_mkvmerge_args(
        cfg: &TrackConfig,
        pair: &EpisodePair,
        active_lang_path: &Path,
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

        let mut args = vec![
            "-o".to_string(),
            out_file.to_string_lossy().to_string(),
        ];

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
        args.push(active_lang_path.to_string_lossy().to_string());

        (out_file, args)
    }

    fn start_merge(&mut self) {
        if self.is_running || self.out_dir.trim().is_empty() {
            self.logs
                .push("Error: Please select an output folder before starting.".into());
            return;
        }
        let _ = fs::create_dir_all(&self.out_dir);

        let jobs: Vec<(usize, EpisodePair)> = self
            .pairs
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, p)| p.enabled)
            .collect();

        if jobs.is_empty() {
            self.logs.push("No episodes enabled for merging.".into());
            return;
        }

        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.is_running = true;
        self.progress = 0.0;
        self.progress_text = format!("Starting batch (0/{})...", jobs.len());

        let bin = self.mkvmerge_bin.clone();
        let cfg = self.cfg.clone();
        let out_dir = self.out_dir.clone();
        let mode = self.naming_mode;
        let custom_name = self.custom_output_name.clone();

        thread::spawn(move || {
            let total_jobs = jobs.len();
            let prog_re = Regex::new(r"(\d+)%").unwrap();

            for (job_i, (idx, pair)) in jobs.into_iter().enumerate() {
                let _ = tx.send(WorkerMsg::EpisodeStatus(idx, "Muxing (0%)".into()));
                let base_progress = job_i as f32 / total_jobs as f32;
                let _ = tx.send(WorkerMsg::Progress(
                    base_progress,
                    format!(
                        "Merging {} ({}/{}) — 0%",
                        pair.ep_key,
                        job_i + 1,
                        total_jobs
                    ),
                ));

                let mut active_lang_path = pair.lang_path.clone();
                let temp_synced_sub = std::env::temp_dir().join(format!("synced_{}.srt", pair.ep_key));

                if cfg.auto_sync_subs {
                    let alass_bin = resolve_bundled_alass();
                    let _ = tx.send(WorkerMsg::Log(format!(
                        "[{}] Running Alass subtitle synchronization...",
                        pair.ep_key
                    )));

                    let mut alass_cmd = Command::new(&alass_bin);
                    alass_cmd.arg(&pair.hq_path)
                             .arg(&pair.lang_path)
                             .arg(&temp_synced_sub);

                    #[cfg(target_os = "windows")]
                    alass_cmd.creation_flags(0x08000000);

                    match alass_cmd.output() {
                        Ok(output) => {
                            if output.status.success() && temp_synced_sub.exists() {
                                active_lang_path = temp_synced_sub.clone();
                                let _ = tx.send(WorkerMsg::Log(format!(
                                    "[{}] Alass sync completed successfully.",
                                    pair.ep_key
                                )));
                            } else {
                                let err_msg = String::from_utf8_lossy(&output.stderr);
                                let _ = tx.send(WorkerMsg::Log(format!(
                                    "[{}] Alass warning/error (falling back to original): {}",
                                    pair.ep_key, err_msg
                                )));
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(WorkerMsg::Log(format!(
                                    "[{}] Failed to execute Alass binary ({}): {}. Skipping sync.",
                                    pair.ep_key, alass_bin, e
                            )));
                        }
                    }
                }

                let (out_file, args) =
                    Self::build_mkvmerge_args(&cfg, &pair, &active_lang_path, &out_dir, mode, &custom_name);

                let _ = tx.send(WorkerMsg::Log(format!(
                    "[{}] Running: {} {}",
                    pair.ep_key,
                    bin,
                    args.join(" ")
                )));

                let mut cmd = Command::new(&bin);
                cmd.args(&args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                #[cfg(target_os = "windows")]
                cmd.creation_flags(0x08000000);

                match cmd.spawn() {
                    Ok(mut child) => {
                        let mut last_lines = Vec::new();
                        if let Some(stdout) = child.stdout.take() {
                            let reader = BufReader::new(stdout);
                            for line in reader.lines().flatten() {
                                if let Some(cap) = prog_re.captures(&line) {
                                    if let Ok(pct) = cap[1].parse::<u32>() {
                                        let pct_clamped = pct.min(100);
                                        let overall = (job_i as f32 + (pct_clamped as f32 / 100.0))
                                            / total_jobs as f32;
                                        let _ = tx.send(WorkerMsg::EpisodeStatus(
                                            idx,
                                            format!("Muxing ({}%)", pct_clamped),
                                        ));
                                        let _ = tx.send(WorkerMsg::Progress(
                                            overall,
                                            format!(
                                                "Merging {} ({}/{}) — {}%",
                                                pair.ep_key,
                                                job_i + 1,
                                                total_jobs,
                                                pct_clamped
                                            ),
                                        ));
                                    }
                                } else if !line.trim().is_empty() {
                                    last_lines.push(line);
                                }
                            }
                        }

                        match child.wait() {
                            Ok(status) => {
                                let code = status.code().unwrap_or(2);
                                if code == 0 || code == 1 {
                                    if cfg.verify_integrity {
                                        let mut verify_cmd = Command::new(&bin);
                                        verify_cmd.arg("--identify").arg(&out_file);
                                        
                                        match verify_cmd.output() {
                                            Ok(out) => {
                                                let stdout_str = String::from_utf8_lossy(&out.stdout);
                                                if stdout_str.contains("Track ID") {
                                                    let _ = tx.send(WorkerMsg::Log(format!(
                                                        "[{}] Integrity check passed.",
                                                        pair.ep_key
                                                    )));
                                                } else {
                                                    let _ = tx.send(WorkerMsg::Log(format!(
                                                        "[{}] Warning: Integrity check response unusual.",
                                                        pair.ep_key
                                                    )));
                                                }
                                            }
                                            Err(e) => {
                                                let _ = tx.send(WorkerMsg::Log(format!(
                                                    "[{}] Integrity check execution failed: {}",
                                                    pair.ep_key, e
                                                )));
                                            }
                                        }
                                    }

                                    let status_label =
                                        if code == 0 { "Done" } else { "Done (Warn)" };
                                    let _ =
                                        tx.send(WorkerMsg::EpisodeStatus(idx, status_label.into()));
                                    let _ = tx.send(WorkerMsg::Log(format!(
                                        "[{}] Saved -> {}",
                                        pair.ep_key,
                                        out_file.display()
                                    )));
                                } else {
                                    let _ = tx.send(WorkerMsg::EpisodeStatus(idx, "Failed".into()));
                                    let _ = tx.send(WorkerMsg::Log(format!(
                                        "[{}] mkvmerge error: {}",
                                        pair.ep_key,
                                        last_lines.join(" | ")
                                    )));
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(WorkerMsg::EpisodeStatus(idx, "Error".into()));
                                let _ = tx.send(WorkerMsg::Log(format!(
                                    "[{}] Wait error: {}",
                                    pair.ep_key, e
                                )));
                            }
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
            let _ = tx.send(WorkerMsg::Progress(
                1.0,
                format!("Completed {}/{} episode(s)", total_jobs, total_jobs),
            ));
            let _ = tx.send(WorkerMsg::Finished);
        });
    }
}

fn load_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("../assets/animerger.png");
    let image = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon path")
        .to_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
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
                    WorkerMsg::Progress(val, text) => {
                        self.progress = val;
                        self.progress_text = text;
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
                    if ui
                        .add(egui::TextEdit::singleline(&mut self.hq_dir).desired_width(520.0))
                        .lost_focus()
                    {
                        self.refresh_file_lists();
                    }
                    if ui.button("Browse...").clicked() {
                        if let Some(p) = rfd::FileDialog::new().pick_folder() {
                            self.hq_dir = p.display().to_string();
                            self.match_folders();
                        }
                    }
                    ui.end_row();

                    ui.label("Right Language Folder:");
                    if ui
                        .add(egui::TextEdit::singleline(&mut self.lang_dir).desired_width(520.0))
                        .lost_focus()
                    {
                        self.refresh_file_lists();
                    }
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
                        ui.radio_value(
                            &mut self.naming_mode,
                            OutputNamingMode::HqName,
                            "HQ File Name",
                        );
                        ui.radio_value(
                            &mut self.naming_mode,
                            OutputNamingMode::LangName,
                            "Right Language File Name",
                        );
                        ui.radio_value(
                            &mut self.naming_mode,
                            OutputNamingMode::CustomName,
                            "Custom Name",
                        );

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
                    if ui.button("Auto-Match Folders").clicked() {
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
                                .hint_text("blank = all languages")
                                .desired_width(110.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.cfg.hq_subs, "Subtitles — Lang filter:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cfg.hq_sub_langs)
                                .hint_text("blank = all languages")
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
                                .hint_text("blank = all languages")
                                .desired_width(110.0),
                        );
                        ui.checkbox(&mut self.cfg.lang_audio_default, "Default");
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.cfg.lang_subs, "Subtitles — Lang filter:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cfg.lang_sub_langs)
                                .hint_text("blank = all languages")
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

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.checkbox(&mut self.cfg.verify_integrity, "Verify file integrity post-mux");
                ui.add_space(16.0);
                ui.checkbox(&mut self.cfg.auto_sync_subs, "Auto-sync subtitles with Alass");
            });

            ui.separator();

            let matched_hq: HashSet<PathBuf> =
                self.pairs.iter().map(|p| p.hq_path.clone()).collect();
            let matched_lang: HashSet<PathBuf> =
                self.pairs.iter().map(|p| p.lang_path.clone()).collect();

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong("Manual Match:");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_pair = !self.is_running
                            && self.selected_manual_hq.is_some()
                            && self.selected_manual_lang.is_some();

                        if ui
                            .add_enabled(can_pair, egui::Button::new("+ Pair Selected"))
                            .clicked()
                        {
                            self.add_manual_pair();
                        }

                        ui.add(
                            egui::TextEdit::singleline(&mut self.manual_ep_key)
                                .desired_width(55.0)
                                .hint_text("E01"),
                        );
                        ui.label("Ep Tag:");
                    });
                });

                ui.add_space(4.0);

                let combo_width = ((ui.available_width() - 24.0) / 2.0).max(160.0);
                let mut hq_changed = false;
                let mut lang_changed = false;

                let selected_hq_text = self
                    .selected_manual_hq
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or("Select HQ File (Source A)...")
                    .to_string();

                let selected_lang_text = self
                    .selected_manual_lang
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or("Select Language File (Source B)...")
                    .to_string();

                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_source("manual_hq_combo")
                        .width(combo_width)
                        .selected_text(&selected_hq_text)
                        .truncate()
                        .show_ui(ui, |ui| {
                            ui.set_min_width(combo_width);
                            for path in &self.all_hq_files {
                                let name = path
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Unknown");
                                let is_matched = matched_hq.contains(path);
                                let label = if is_matched {
                                    format!("[Matched] {}", name)
                                } else {
                                    format!("[Unmatched] {}", name)
                                };
                                if ui
                                    .selectable_value(
                                        &mut self.selected_manual_hq,
                                        Some(path.clone()),
                                        &label,
                                    )
                                    .on_hover_text(name)
                                    .clicked()
                                {
                                    hq_changed = true;
                                }
                            }
                        })
                        .response
                        .on_hover_text(&selected_hq_text);

                    egui::ComboBox::from_id_source("manual_lang_combo")
                        .width(combo_width)
                        .selected_text(&selected_lang_text)
                        .truncate()
                        .show_ui(ui, |ui| {
                            ui.set_min_width(combo_width);
                            for path in &self.all_lang_files {
                                let name = path
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Unknown");
                                let is_matched = matched_lang.contains(path);
                                let label = if is_matched {
                                    format!("[Matched] {}", name)
                                } else {
                                    format!("[Unmatched] {}", name)
                                };
                                if ui
                                    .selectable_value(
                                        &mut self.selected_manual_lang,
                                        Some(path.clone()),
                                        &label,
                                    )
                                    .on_hover_text(name)
                                    .clicked()
                                {
                                    lang_changed = true;
                                }
                            }
                        })
                        .response
                        .on_hover_text(&selected_lang_text);
                });

                if hq_changed || lang_changed {
                    self.suggest_manual_ep_key();
                }
            });

            ui.add_space(4.0);

            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("Matched Episodes ({})", self.pairs.len()));
                if ui
                    .add_enabled(
                        !self.is_running && !self.pairs.is_empty(),
                        egui::Button::new("Unmatch All"),
                    )
                    .clicked()
                {
                    self.pairs.clear();
                    self.logs.push("Cleared all matched pairs.".into());
                    self.update_default_manual_selection();
                }

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

            if self.is_running || self.progress > 0.0 {
                ui.add_space(2.0);
                ui.add(
                    egui::ProgressBar::new(self.progress)
                        .text(&self.progress_text)
                        .animate(self.is_running),
                );
                ui.add_space(2.0);
            }

            let mut remove_idx: Option<usize> = None;

            egui::ScrollArea::vertical()
                .id_source("pairs_scroll")
                .max_height(175.0)
                .show(ui, |ui| {
                    egui::Grid::new("pairs_grid")
                        .striped(true)
                        .num_columns(7)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.strong("Use");
                            ui.strong("Ep");
                            ui.strong("High Quality File (Source A)");
                            ui.strong("Language File (Source B)");
                            ui.strong("Sync (ms)");
                            ui.strong("Status");
                            ui.strong("Action");
                            ui.end_row();

                            for (idx, pair) in self.pairs.iter_mut().enumerate() {
                                ui.checkbox(&mut pair.enabled, "");
                                ui.add(
                                    egui::TextEdit::singleline(&mut pair.ep_key)
                                        .desired_width(45.0),
                                );
                                let hq_name = pair
                                    .hq_path
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("");
                                ui.add(egui::Label::new(hq_name).truncate())
                                    .on_hover_text(hq_name);

                                let lang_name = pair
                                    .lang_path
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("");
                                ui.add(egui::Label::new(lang_name).truncate())
                                    .on_hover_text(lang_name);

                                ui.add(
                                    egui::DragValue::new(&mut pair.delay_ms)
                                        .speed(50)
                                        .suffix(" ms"),
                                );
                                ui.label(&pair.status);
                                if ui
                                    .add_enabled(!self.is_running, egui::Button::new("Unmatch"))
                                    .clicked()
                                {
                                    remove_idx = Some(idx);
                                }
                                ui.end_row();
                            }
                        });
                });

            if let Some(idx) = remove_idx {
                let removed = self.pairs.remove(idx);
                self.logs.push(format!(
                    "Unmatched [{}]: {} <-> {}",
                    removed.ep_key,
                    removed
                        .hq_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(""),
                    removed
                        .lang_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                ));
                self.update_default_manual_selection();
            }

            ui.separator();

            ui.strong("mkvmerge Output Log");
            egui::ScrollArea::vertical()
                .id_source("logs_scroll")
                .stick_to_bottom(true)
                .max_height(100.0)
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
            .with_inner_size([1080.0, 760.0])
            .with_min_inner_size([900.0, 640.0])
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "AniMerger",
        options,
        Box::new(|_cc| Ok(Box::new(AniMergerApp::default()))),
    )
}