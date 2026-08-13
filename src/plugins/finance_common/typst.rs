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

const MAIN_FILE: &str = "invoice.typ";

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
    let _ = std::fs::remove_dir_all(&dir);
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
}
