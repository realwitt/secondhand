//! Secondhand's converter: finds FreeHand drawings in what the user dropped, converts each one with
//! the VectorCraft engine, and writes the result next to the original without overwriting anything.
//!
//! Nothing here touches the network, and the original files are only ever read.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

/// Files larger than this are not read (VectorCraft's FreeHand reader refuses more than 64 MiB).
const MAX_INPUT: u64 = 64 << 20;
/// How much of a file without a FreeHand extension is read to see whether it is one.
const SNIFF: u64 = 1 << 20;
/// Folder recursion stops this deep, so a link loop or a pathological tree can't run forever.
const MAX_DEPTH: usize = 64;

/// What to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// A PDF-compatible Illustrator file (`.ai`): a PDF that Illustrator opens directly.
    Ai,
    /// A PDF with one page per FreeHand page and layers as PDF layers.
    Pdf,
    /// One SVG per page.
    Svg,
}

impl Format {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().trim_start_matches('.') {
            "ai" => Some(Self::Ai),
            "pdf" => Some(Self::Pdf),
            "svg" => Some(Self::Svg),
            _ => None,
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            Self::Ai => "ai",
            Self::Pdf => "pdf",
            Self::Svg => "svg",
        }
    }
}

/// How one file went.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Outcome {
    /// Written. `outputs` are the new files; `notes` say what didn't come across exactly.
    Converted { input: PathBuf, outputs: Vec<PathBuf>, notes: Vec<String> },
    /// Not converted, on purpose: not a FreeHand file, or a FreeHand version that can't be read.
    Skipped { input: PathBuf, reason: String },
    /// Something went wrong with this file. The rest of the batch carries on.
    Failed { input: PathBuf, reason: String },
}

impl Outcome {
    pub fn input(&self) -> &Path {
        match self {
            Self::Converted { input, .. } | Self::Skipped { input, .. } | Self::Failed { input, .. } => input,
        }
    }
}

/// Progress reported while a batch runs.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum Event {
    /// The batch has been listed: `total` files will be tried.
    Start { total: usize },
    /// File `index` (0-based) is being converted.
    Begin { index: usize, total: usize, input: PathBuf },
    /// File `index` is done.
    Done { index: usize, total: usize, outcome: Outcome },
}

/// The end of a batch.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Summary {
    pub converted: Vec<Outcome>,
    pub skipped: Vec<Outcome>,
    pub failed: Vec<Outcome>,
    /// Files inside dropped folders that aren't FreeHand drawings and were left alone.
    pub ignored_in_folders: usize,
    /// The folder of the first file written, for "Open output folder".
    pub output_folder: Option<PathBuf>,
}

impl Summary {
    pub fn push(&mut self, o: Outcome) {
        match o {
            Outcome::Converted { ref outputs, .. } => {
                if self.output_folder.is_none() {
                    self.output_folder = outputs.first().and_then(|p| p.parent().map(Path::to_path_buf));
                }
                self.converted.push(o)
            }
            Outcome::Skipped { .. } => self.skipped.push(o),
            Outcome::Failed { .. } => self.failed.push(o),
        }
    }

    /// A plain-English report, one line per file.
    pub fn report(&self) -> String {
        let mut s = String::new();
        let n = |k: usize, one: &str, many: &str| if k == 1 { format!("1 {one}") } else { format!("{k} {many}") };
        s += &format!(
            "{} converted, {} skipped, {} failed.\n",
            n(self.converted.len(), "file", "files"),
            self.skipped.len(),
            self.failed.len()
        );
        for o in &self.converted {
            if let Outcome::Converted { input, outputs, notes } = o {
                let outs: Vec<String> = outputs.iter().map(|p| file_name(p)).collect();
                s += &format!("Converted: {} → {}\n", input.display(), outs.join(", "));
                for note in notes {
                    s += &format!("    note: {note}\n");
                }
            }
        }
        for o in self.skipped.iter().chain(&self.failed) {
            match o {
                Outcome::Skipped { input, reason } => s += &format!("Skipped: {} — {reason}\n", input.display()),
                Outcome::Failed { input, reason } => s += &format!("Failed: {} — {reason}\n", input.display()),
                Outcome::Converted { .. } => {}
            }
        }
        if self.ignored_in_folders > 0 {
            s += &if self.ignored_in_folders == 1 {
                "1 other file in the folders wasn't a FreeHand drawing and was left alone.\n".to_string()
            } else {
                format!("{} other files in the folders weren't FreeHand drawings and were left alone.\n", self.ignored_in_folders)
            };
        }
        s
    }
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

/// True for the names FreeHand gave its documents and templates: `.fh`, `.fh3` … `.fh11`, `.ft*`.
pub fn has_freehand_extension(p: &Path) -> bool {
    let Some(ext) = p.extension().and_then(|e| e.to_str()) else { return false };
    let ext = ext.to_ascii_lowercase();
    let Some(rest) = ext.strip_prefix("fh").or_else(|| ext.strip_prefix("ft")) else { return false };
    rest.is_empty() || (rest.len() <= 2 && rest.bytes().all(|b| b.is_ascii_digit()))
}

/// Extensions that are certainly something else, so files inside dropped folders aren't even opened.
fn obviously_other(p: &Path) -> bool {
    const OTHER: &[&str] = &[
        "ai", "pdf", "svg", "eps", "ps", "jpg", "jpeg", "png", "gif", "tif", "tiff", "bmp", "psd", "txt", "rtf", "doc", "docx", "xls",
        "xlsx", "zip", "sit", "sitx", "dmg", "app", "exe", "html", "htm", "mp3", "mp4", "mov", "wav", "ttf", "otf", "dfont", "indd",
        "qxd", "cdr", "swf", "fla", "ds_store", "ini", "db", "lnk",
    ];
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| OTHER.contains(&e.to_ascii_lowercase().as_str()))
}

fn is_hidden(p: &Path) -> bool {
    p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.') || n == "__MACOSX")
}

/// Reads the start of `p` and asks the FreeHand reader whether it recognises the header, including
/// old versions it refuses (so they are reported rather than ignored).
fn looks_like_freehand(p: &Path) -> bool {
    let Ok(f) = File::open(p) else { return false };
    let mut head = Vec::new();
    if f.take(SNIFF).read_to_end(&mut head).is_err() {
        return false;
    }
    vectorcraft_freehand::is_freehand(&head)
}

/// What was found in the dropped paths.
#[derive(Debug, Default)]
pub struct Listing {
    /// Files to try, in a stable order, each once.
    pub files: Vec<PathBuf>,
    /// Dropped paths that could not be used (missing, unreadable folder).
    pub problems: Vec<Outcome>,
    /// Files inside folders that aren't FreeHand drawings.
    pub ignored_in_folders: usize,
}

/// Expands `inputs` (files and folders, as dropped) into the files to convert. Folders are walked
/// recursively; inside them only FreeHand files are picked (by extension, or by their header when
/// they have no telling extension, as old Mac files often don't). Dropped files are always tried.
pub fn list(inputs: &[PathBuf]) -> Listing {
    let mut out = Listing::default();
    let mut seen = std::collections::HashSet::new();
    for input in inputs {
        match fs::metadata(input) {
            Err(e) => out.problems.push(Outcome::Failed { input: input.clone(), reason: format!("couldn't find or open it ({})", plain_io(&e)) }),
            Ok(m) if m.is_dir() => walk(input, 0, &mut out, &mut seen),
            Ok(_) => {
                if seen.insert(canonical(input)) {
                    out.files.push(input.clone());
                }
            }
        }
    }
    out
}

fn canonical(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

fn walk(dir: &Path, depth: usize, out: &mut Listing, seen: &mut std::collections::HashSet<PathBuf>) {
    if depth > MAX_DEPTH {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            out.problems.push(Outcome::Failed { input: dir.to_path_buf(), reason: format!("couldn't open this folder ({})", plain_io(&e)) });
            return;
        }
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for p in paths {
        if is_hidden(&p) {
            continue;
        }
        // symlink_metadata: links aren't followed, so a link back up the tree can't loop.
        let Ok(m) = fs::symlink_metadata(&p) else { continue };
        if m.is_dir() {
            walk(&p, depth + 1, out, seen);
        } else if m.is_file() {
            let pick = has_freehand_extension(&p) || (!obviously_other(&p) && m.len() <= MAX_INPUT && looks_like_freehand(&p));
            if pick {
                if seen.insert(canonical(&p)) {
                    out.files.push(p);
                }
            } else {
                out.ignored_in_folders += 1;
            }
        }
    }
}

fn plain_io(e: &std::io::Error) -> String {
    use std::io::ErrorKind::*;
    match e.kind() {
        NotFound => "it isn't there any more".into(),
        PermissionDenied => "permission denied".into(),
        _ => e.to_string(),
    }
}

/// Converts everything in `inputs`, calling `progress` as it goes. One file's failure (even a crash
/// inside the engine) never stops the others.
pub fn run(inputs: &[PathBuf], format: Format, mut progress: impl FnMut(&Event)) -> Summary {
    let listing = list(inputs);
    let mut summary = Summary { ignored_in_folders: listing.ignored_in_folders, ..Summary::default() };
    for p in listing.problems {
        summary.push(p);
    }
    let total = listing.files.len();
    progress(&Event::Start { total });
    for (index, input) in listing.files.into_iter().enumerate() {
        progress(&Event::Begin { index, total, input: input.clone() });
        let outcome = convert_isolated(&input, format);
        progress(&Event::Done { index, total, outcome: outcome.clone() });
        summary.push(outcome);
    }
    summary
}

/// [`convert`] on its own thread with a large stack, with any panic turned into a failure.
pub fn convert_isolated(input: &Path, format: Format) -> Outcome {
    let owned = input.to_path_buf();
    let worker = std::thread::Builder::new().name("convert".into()).stack_size(256 << 20).spawn(move || {
        catch_unwind(AssertUnwindSafe(|| convert(&owned, format)))
    });
    let failed = |reason: &str| Outcome::Failed { input: input.to_path_buf(), reason: reason.into() };
    match worker {
        Err(_) => failed("couldn't start converting (the computer is low on memory)"),
        Ok(handle) => match handle.join() {
            Ok(Ok(outcome)) => outcome,
            _ => failed("the converter hit an internal error on this file; the file may be damaged"),
        },
    }
}

/// Converts one file and writes the result beside it.
pub fn convert(input: &Path, format: Format) -> Outcome {
    let skipped = |reason: String| Outcome::Skipped { input: input.to_path_buf(), reason };
    let failed = |reason: String| Outcome::Failed { input: input.to_path_buf(), reason };

    let size = match fs::metadata(input) {
        Ok(m) => m.len(),
        Err(e) => return failed(format!("couldn't read it ({})", plain_io(&e))),
    };
    if size == 0 {
        return skipped("the file is empty".into());
    }
    if size > MAX_INPUT {
        return failed("the file is larger than 64 MB, which is more than any FreeHand drawing this can read".into());
    }
    let bytes = match fs::read(input) {
        Ok(b) => b,
        Err(e) => return failed(format!("couldn't read it ({})", plain_io(&e))),
    };
    if !vectorcraft_freehand::is_freehand(&bytes) {
        return skipped("this isn't a FreeHand file".into());
    }

    let (docs, notes) = match engine_convert(input, &bytes, format) {
        Ok(v) => v,
        Err(EngineFailure::Unsupported(msg)) => return skipped(msg),
        Err(EngineFailure::Other(msg)) => return failed(msg),
    };

    let Some(dir) = input.parent() else { return failed("couldn't tell which folder it is in".into()) };
    let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Untitled".into());
    let mut outputs = Vec::new();
    let many = docs.len() > 1;
    for (i, (page, data)) in docs.iter().enumerate() {
        let base = if many { format!("{stem} - {}", page.clone().unwrap_or_else(|| format!("Page {}", i + 1))) } else { stem.clone() };
        match write_new(dir, &sanitize(&base), format.extension(), data) {
            Ok(p) => outputs.push(p),
            Err(e) => {
                return failed(format!("couldn't save next to the original ({}){}", plain_io(&e), if outputs.is_empty() { "" } else { "; some pages were saved" }));
            }
        }
    }
    Outcome::Converted { input: input.to_path_buf(), outputs, notes }
}

enum EngineFailure {
    Unsupported(String),
    Other(String),
}

/// Turns the reader's sentences into ones a designer can act on.
fn friendly(err: &str) -> EngineFailure {
    let e = err.trim();
    if let Some(rest) = e.split("unsupported FreeHand version ").nth(1) {
        let version = rest.split(':').next().unwrap_or("").trim();
        return EngineFailure::Unsupported(format!(
            "this file was saved by FreeHand {version}, and only FreeHand 8, 9, 10 and MX files can be converted. If you still have FreeHand MX, open it there and save it again, then drop it here; or export it from FreeHand as EPS or PDF"
        ));
    }
    let tail = e.rsplit(": ").next().unwrap_or(e);
    let msg = match tail {
        t if t.contains("is cut off") => "the file is incomplete (cut off). It may not have finished copying or downloading".to_string(),
        t if t.contains("compressed data is damaged") => "the file is damaged (its compressed data can't be read)".to_string(),
        t if t.contains("not a FreeHand file") => return EngineFailure::Unsupported("this isn't a FreeHand file".into()),
        t if t.contains("claims more") || t.contains("larger than") || t.contains("safety limit") => {
            "the file is damaged or too large to read safely".to_string()
        }
        t if t.contains("size is not known") => "the file uses a part of the FreeHand format this can't read yet".to_string(),
        t => format!("couldn't read the drawing ({t})"),
    };
    EngineFailure::Other(msg)
}

type Pages = Vec<(Option<String>, Vec<u8>)>;

fn engine_convert(input: &Path, bytes: &[u8], format: Format) -> Result<(Pages, Vec<String>), EngineFailure> {
    use vectorcraft_engine::Session;
    let mut s = Session::new();
    let name = input.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "drawing.fh".into());
    let opened = vectorcraft_engine::cmd::fileio::open_bytes(&mut s, &name, bytes, None).map_err(|e| friendly(&e.to_string()))?;
    let mut notes: Vec<String> = strings(&opened["warnings"]);
    // The .ai is a PDF-compatible Illustrator file: a PDF 1.7, one page per artboard, layers as
    // optional content (hidden ones off), without VectorCraft's own editing data attached.
    let pdf = json!({
        "format": "pdf", "range": "all", "compatibility": "1.7", "createLayers": true, "includeNonPrinting": true,
        "preserveEditing": false, "hiddenLayers": true,
    });
    let out = match format {
        Format::Ai | Format::Pdf => s.execute("document.serialize", &pdf),
        Format::Svg => s.execute("document.serialize", &json!({"format": "svg", "range": "all", "hiddenLayers": true})),
    }
    .map_err(|e| EngineFailure::Other(format!("couldn't write the {} ({e})", format.extension().to_uppercase())))?;
    notes.extend(strings(&out["warnings"]));
    notes.dedup();
    let decode = |v: &Value| -> Option<Vec<u8>> {
        if let Some(b) = v["dataBase64"].as_str() {
            vectorcraft_format::base64_decode(b)
        } else {
            v["text"].as_str().map(|t| t.as_bytes().to_vec())
        }
    };
    let mut pages = Vec::new();
    if let Some(files) = out["files"].as_array().filter(|f| f.len() > 1 && format == Format::Svg) {
        let names: Vec<Option<String>> = artboard_names(&s);
        for (i, f) in files.iter().enumerate() {
            let data = decode(f).ok_or_else(|| EngineFailure::Other("the converter returned no data".into()))?;
            pages.push((names.get(i).cloned().flatten(), data));
        }
    } else {
        let data = decode(&out).filter(|d| !d.is_empty()).ok_or_else(|| EngineFailure::Other("the converter returned no data".into()))?;
        pages.push((None, data));
    }
    Ok((pages, notes))
}

fn artboard_names(s: &vectorcraft_engine::Session) -> Vec<Option<String>> {
    s.doc().map(|d| d.doc.artboards.iter().map(|a| Some(a.name.clone()).filter(|n| !n.trim().is_empty())).collect()).unwrap_or_default()
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|w| w.as_str().map(str::to_owned)).collect()).unwrap_or_default()
}

/// Keeps a file name legal on macOS and Windows.
fn sanitize(name: &str) -> String {
    let s: String = name.chars().map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() { '_' } else { c }).collect();
    let s = s.trim().trim_end_matches('.').to_string();
    if s.is_empty() { "Untitled".into() } else { s }
}

/// Writes `data` to `dir/stem.ext`, or `stem (1).ext`, `stem (2).ext`… — never over an existing file.
/// The name is claimed atomically (`create_new`), so two runs can't pick the same one.
pub fn write_new(dir: &Path, stem: &str, ext: &str, data: &[u8]) -> std::io::Result<PathBuf> {
    for n in 0..10_000u32 {
        let name = if n == 0 { format!("{stem}.{ext}") } else { format!("{stem} ({n}).{ext}") };
        let path = dir.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => {
                let result = f.write_all(data).and_then(|()| f.sync_all());
                if let Err(e) = result {
                    drop(f);
                    let _ = fs::remove_file(&path);
                    return Err(e);
                }
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::other("too many files with that name already"))
}

#[cfg(test)]
mod tests;
