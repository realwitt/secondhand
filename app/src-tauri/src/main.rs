//! Lifeboat's desktop app: one window to drop FreeHand files and folders on, plus the macOS
//! droplet (files dropped on the app icon) and a headless `--batch` mode for scripts and CI.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};

use lifeboat_core::{Event, Format, Summary};
use serde::Serialize;
use tauri::{AppHandle, Emitter, WindowEvent};
#[cfg(target_os = "macos")]
use tauri::{Manager, RunEvent};

#[derive(Default)]
struct Shared {
    queue: VecDeque<(Vec<PathBuf>, Format)>,
    /// The events of the batch being run (or last run), so a window that loads late can catch up.
    events: Vec<Event>,
    summary: Option<Summary>,
    format: Option<Format>,
}

#[derive(Clone, Default)]
struct State(Arc<(Mutex<Shared>, Condvar)>);

impl State {
    fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.0.0.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn enqueue(&self, paths: Vec<PathBuf>, format: Option<Format>) {
        if paths.is_empty() {
            return;
        }
        let mut s = self.lock();
        let f = format.or(s.format).unwrap_or(Format::Ai);
        s.queue.push_back((paths, f));
        self.0.1.notify_one();
    }
}

#[derive(Serialize)]
struct Status {
    events: Vec<Event>,
    summary: Option<Summary>,
    format: Option<Format>,
}

#[tauri::command]
fn convert(state: tauri::State<'_, State>, paths: Vec<String>, format: String) {
    let f = Format::parse(&format);
    if let Some(f) = f {
        state.lock().format = Some(f);
    }
    state.enqueue(paths.into_iter().map(PathBuf::from).collect(), f);
}

#[tauri::command]
fn set_format(state: tauri::State<'_, State>, format: String) {
    if let Some(f) = Format::parse(&format) {
        state.lock().format = Some(f);
    }
}

#[tauri::command]
fn status(state: tauri::State<'_, State>) -> Status {
    let s = state.lock();
    Status { events: s.events.clone(), summary: s.summary.clone(), format: s.format }
}

#[tauri::command]
fn report(state: tauri::State<'_, State>) -> String {
    state.lock().summary.as_ref().map(Summary::report).unwrap_or_default()
}

#[tauri::command]
fn open_folder(app: AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_path(path, None::<&str>).map_err(|e| e.to_string())
}

/// Runs queued batches one after another, forever, on its own thread.
fn worker(app: AppHandle, state: State) {
    loop {
        let (paths, format) = {
            let mut s = state.lock();
            loop {
                if let Some(job) = s.queue.pop_front() {
                    break job;
                }
                s = state.0.1.wait(s).unwrap_or_else(|e| e.into_inner());
            }
        };
        let summary = lifeboat_core::run(&paths, format, |e| {
            {
                let mut s = state.lock();
                if matches!(e, Event::Start { .. }) {
                    s.events.clear();
                    s.summary = None;
                }
                s.events.push(e.clone());
            }
            let _ = app.emit("lifeboat://event", e);
        });
        state.lock().summary = Some(summary.clone());
        let _ = app.emit("lifeboat://summary", &summary);
    }
}

/// Paths given on the command line (Windows "Open with", files dropped on the .exe).
fn arg_paths(args: &[String]) -> Vec<PathBuf> {
    args.iter().filter(|a| !a.starts_with("-psn_") && !a.starts_with("--")).map(PathBuf::from).collect()
}

/// `Lifeboat --batch [--format ai|pdf|svg] [--report REPORT.json] PATH…`: converts without a
/// window and exits 0 when nothing failed, 1 otherwise. The report is a JSON file, because a
/// Windows GUI program has no console to print to.
fn batch(args: &[String]) -> i32 {
    let mut format = Format::Ai;
    let mut report = None::<PathBuf>;
    let mut paths = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--batch" => {}
            "--format" => format = it.next().and_then(|f| Format::parse(f)).unwrap_or(Format::Ai),
            "--report" => report = it.next().map(PathBuf::from),
            p => paths.push(PathBuf::from(p)),
        }
    }
    let summary = lifeboat_core::run(&paths, format, |_| {});
    let text = summary.report();
    print!("{text}");
    if let Some(r) = report {
        let json = serde_json::to_string_pretty(&summary).unwrap_or_default();
        let _ = std::fs::write(&r, json);
        let _ = std::fs::write(r.with_extension("txt"), text);
    }
    i32::from(!summary.failed.is_empty())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--batch") {
        std::process::exit(batch(&args));
    }
    let state = State::default();
    let startup = arg_paths(&args);
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![convert, set_format, status, report, open_folder])
        .setup({
            let state = state.clone();
            move |app| {
                let handle = app.handle().clone();
                let s = state.clone();
                std::thread::Builder::new().name("batches".into()).spawn(move || worker(handle, s))?;
                state.enqueue(startup, None);
                Ok(())
            }
        })
        .on_window_event({
            let state = state.clone();
            move |_, event| {
                if let WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
                    state.enqueue(paths.clone(), None);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Lifeboat couldn't start its window");
    #[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
    app.run(move |handle, event| {
        // macOS: files and folders dropped on the app icon, or opened with Lifeboat.
        #[cfg(target_os = "macos")]
        if let RunEvent::Opened { urls } = &event {
            let paths: Vec<PathBuf> = urls.iter().filter_map(|u| u.to_file_path().ok()).collect();
            state.enqueue(paths, None);
            if let Some(w) = handle.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
    });
}
