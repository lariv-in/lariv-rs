//! Compile Typst markup to PDF via the [`typst`](https://docs.rs/typst/latest/typst/) library.

use std::path::{Path, PathBuf};

use typst::diag::FileResult;
use typst::foundations::{Bytes, Datetime, Duration};
use typst::text::Font;
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::datetime::Time;
use typst_kit::files::{FileLoader, FileStore, FsRoot};
use typst_kit::fonts::FontStore;
use typst_syntax::{FileId, RootedPath, VirtualPath, VirtualRoot};

const MAIN_FILE: &str = "main.typ";

/// Fresh temp directory for one Typst compile (source + downloaded assets).
pub fn typst_work_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "lariv-typst-{}-{}",
        std::process::id(),
        uuid_simple()
    ))
}

/// Compile Typst source to PDF bytes using the Typst compiler crate.
///
/// `work_dir` must already exist; relative `#image(...)` paths resolve against it.
pub async fn typst_compile_in(work_dir: &Path, source: &str) -> Result<Vec<u8>, String> {
    let work_dir = work_dir.to_path_buf();
    let source = source.to_string();
    tokio::task::spawn_blocking(move || typst_compile_in_blocking(&work_dir, &source))
        .await
        .map_err(|e| format!("typst compile task failed: {e}"))?
}

/// Compile Typst source in a fresh temp directory (no co-located assets).
pub async fn typst_compile(source: &str) -> Result<Vec<u8>, String> {
    let dir = typst_work_dir();
    let result = typst_compile_in(&dir, source).await;
    if let Err(e) = std::fs::remove_dir_all(&dir) {
        tracing::warn!(error = %e, path = %dir.display(), "failed to remove typst work dir");
    }
    result
}

fn typst_compile_in_blocking(work_dir: &Path, source: &str) -> Result<Vec<u8>, String> {
    std::fs::create_dir_all(work_dir).map_err(|e| format!("create typst temp dir: {e}"))?;
    let typ_path = work_dir.join(MAIN_FILE);
    std::fs::write(&typ_path, source).map_err(|e| format!("write typst source: {e}"))?;

    let world = CompileWorld::new(work_dir, MAIN_FILE)?;
    let result = typst::compile(&world);
    comemo::evict(30);

    let document = result
        .output
        .map_err(|diagnostics| format_typst_diagnostics(&diagnostics))?;

    typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|diagnostics| format_typst_diagnostics(&diagnostics))
}

struct CompileWorld {
    library: LazyHash<Library>,
    fonts: FontStore,
    files: FileStore<ProjectFiles>,
    now: Time,
}

impl CompileWorld {
    fn new(work_dir: &Path, main_file: &str) -> Result<Self, String> {
        let root = work_dir
            .canonicalize()
            .map_err(|e| format!("resolve typst work dir: {e}"))?;
        let main_path = root.join(main_file);
        if !main_path.is_file() {
            return Err(format!(
                "typst main file not found: {}",
                main_path.display()
            ));
        }

        let vpath = VirtualPath::virtualize(&root, &main_path)
            .map_err(|e| format!("virtualize typst main path: {e}"))?;
        let main = RootedPath::new(VirtualRoot::Project, vpath).intern();

        let mut fonts = FontStore::new();
        fonts.extend(typst_kit::fonts::embedded());
        fonts.extend(typst_kit::fonts::system());

        Ok(Self {
            library: LazyHash::new(Library::default()),
            fonts,
            files: FileStore::new(ProjectFiles {
                main,
                project: FsRoot::new(root),
            }),
            now: Time::system(),
        })
    }
}

struct ProjectFiles {
    main: FileId,
    project: FsRoot,
}

impl FileLoader for ProjectFiles {
    fn load(&self, id: FileId) -> FileResult<Bytes> {
        match id.root() {
            VirtualRoot::Project => self.project.load(id.vpath()),
            VirtualRoot::Package(_) => Err(typst::diag::FileError::NotFound(
                id.vpath().get_with_slash().into(),
            )),
        }
    }
}

impl World for CompileWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<typst::text::FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.files.loader().main
    }

    fn source(&self, id: FileId) -> FileResult<typst_syntax::Source> {
        self.files.source(id)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.file(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.now.today(offset)
    }
}

fn format_typst_diagnostics(
    diagnostics: &typst::diag::EcoVec<typst::diag::SourceDiagnostic>,
) -> String {
    diagnostics
        .iter()
        .map(|d| d.message.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Turn a postal address into Typst markup with real line breaks.
///
/// A newline in Typst markup is only a space. A line that starts with `- ` is a
/// bullet list, so a wrapped `Dist. - Pune` becomes a bullet and the following
/// lines stay indented under it. Continuation lines that start with `- `, `+ `,
/// or `/ ` are joined back onto the previous line. Other newlines become
/// Typst line breaks (`\`). A line that already ends with a line break is kept.
pub fn typst_address_lines(markup: &str) -> String {
    let mut logical: Vec<String> = Vec::new();
    for raw in markup.split('\n') {
        let body = raw.trim();
        if body.is_empty() {
            continue;
        }
        if is_typst_continuation(body) {
            if let Some(prev) = logical.last_mut() {
                if !prev.ends_with(' ') {
                    prev.push(' ');
                }
                prev.push_str(body);
                continue;
            }
        }
        logical.push(body.to_string());
    }
    let mut out = String::new();
    for (i, line) in logical.iter().enumerate() {
        if i > 0 {
            if !ends_with_typst_break(&logical[i - 1]) {
                out.push_str(" \\");
            }
            out.push('\n');
        }
        out.push_str(line);
    }
    out
}

fn is_typst_continuation(line: &str) -> bool {
    let mut chars = line.chars();
    match chars.next() {
        Some('-' | '+' | '/') => matches!(chars.next(), Some(' ' | '\t') | None),
        _ => false,
    }
}

fn ends_with_typst_break(line: &str) -> bool {
    let line = line.trim_end();
    line.ends_with('\\') && !line.ends_with("\\\\")
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn typst_compiles_minimal_document() {
        let pdf = typst_compile("Hello, world!").await.expect("compile");
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn address_joins_wrapped_dash_and_keeps_plain_newlines() {
        let src = "Plot No. : D-5, D-7, D-9, C-63, C-1 & F5 \\ MIDC Jejuri – 412 303, Dist.\n\
                   - Pune, Maharashtra, India \\ Pune 412303 \\ Maharastra \\ India";
        assert_eq!(
            typst_address_lines(src),
            "Plot No. : D-5, D-7, D-9, C-63, C-1 & F5 \\ MIDC Jejuri – 412 303, Dist. - Pune, Maharashtra, India \\ Pune 412303 \\ Maharastra \\ India"
        );
        assert_eq!(
            typst_address_lines("Gat No. 427, Tal\nAlandi Fata, Khed"),
            "Gat No. 427, Tal \\\nAlandi Fata, Khed"
        );
        assert_eq!(
            typst_address_lines("Industrial Area, \\\nPune 411019 \\"),
            "Industrial Area, \\\nPune 411019 \\"
        );
    }
}
