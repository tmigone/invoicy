use std::sync::{LazyLock, OnceLock};

use chrono::Datelike;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

static LIBRARY: LazyLock<LazyHash<Library>> = LazyLock::new(|| LazyHash::new(Library::default()));

// Embedded assets
static ARCA_JPEG: &[u8] = include_bytes!("../assets/arca.jpeg");

pub struct InvoiceWorld {
    source: Source,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    now: OnceLock<Option<Datetime>>,
}

impl InvoiceWorld {
    pub fn new(source_text: &str) -> Self {
        let source = Source::new(
            FileId::new(None, VirtualPath::new("main.typ")),
            source_text.to_string(),
        );

        let (book, fonts) = load_fonts();

        Self {
            source,
            book: LazyHash::new(book),
            fonts,
            now: OnceLock::new(),
        }
    }
}

impl World for InvoiceWorld {
    fn library(&self) -> &LazyHash<Library> {
        &LIBRARY
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = id.vpath().as_rootless_path();
        let filename = path.to_string_lossy();

        // Check embedded assets
        match filename.as_ref() {
            "arca.jpeg" => Ok(Bytes::new(ARCA_JPEG.to_vec())),
            _ => Err(FileError::NotFound(path.into())),
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        *self.now.get_or_init(|| {
            let now = chrono::Local::now();
            Datetime::from_ymd(
                now.year(),
                now.month().try_into().ok()?,
                now.day().try_into().ok()?,
            )
        })
    }
}

fn load_fonts() -> (FontBook, Vec<Font>) {
    let mut book = FontBook::new();
    let mut fonts = Vec::new();

    let font_dirs = get_system_font_dirs();

    for dir in font_dirs {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if is_font_file(&path) {
                    if let Ok(data) = std::fs::read(&path) {
                        let bytes = Bytes::new(data);
                        for font in Font::iter(bytes) {
                            book.push(font.info().clone());
                            fonts.push(font);
                        }
                    }
                }
            }
        }
    }

    if fonts.is_empty() {
        eprintln!("Warning: No system fonts found. PDF may not render text correctly.");
    }

    (book, fonts)
}

fn get_system_font_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(target_os = "macos")]
    {
        dirs.push("/System/Library/Fonts".into());
        dirs.push("/Library/Fonts".into());
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join("Library/Fonts"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        dirs.push("/usr/share/fonts".into());
        dirs.push("/usr/local/share/fonts".into());
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".fonts"));
            dirs.push(home.join(".local/share/fonts"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(windir) = std::env::var_os("WINDIR") {
            dirs.push(std::path::PathBuf::from(windir).join("Fonts"));
        }
    }

    dirs
}

fn is_font_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_lowercase().as_str(), "ttf" | "otf" | "ttc" | "otc"))
        .unwrap_or(false)
}

pub fn compile_to_pdf(source: &str) -> Result<Vec<u8>, String> {
    let world = InvoiceWorld::new(source);

    let result = typst::compile(&world);

    if !result.warnings.is_empty() {
        for warning in &result.warnings {
            eprintln!("Warning: {}", warning.message);
        }
    }

    let document = result.output.map_err(|errors| {
        errors
            .iter()
            .map(|e| format!("{:?}: {}", e.severity, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    let options = typst_pdf::PdfOptions::default();
    let pdf = typst_pdf::pdf(&document, &options).map_err(|errors| {
        errors
            .iter()
            .map(|e| format!("{:?}: {}", e.severity, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    Ok(pdf)
}
