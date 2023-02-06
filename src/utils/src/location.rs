use std::{fmt::Display, path::PathBuf};

use crate::filesystem::FileSystem;
use rustc_lint::{LateContext, LintContext};
use rustc_span::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawLoc {
    pub filename: rustc_span::FileName,
    pub lines: Vec<rustc_span::LineInfo>,
}

unsafe impl Send for RawLoc {}

impl RawLoc {
    pub fn new<'a>(ctx: &LateContext<'a>, span: Span) -> Self {
        let sess = ctx.sess();
        let source_map = sess.source_map();
        let lines = source_map.span_to_lines(span).expect("").lines;
        let filename = source_map.span_to_filename(span);
        Self { lines, filename }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Loc(PathBuf, String);

impl Loc {
    pub fn path(&self) -> &PathBuf {
        &self.0
    }

    pub fn fn_name(&self) -> &str {
        self.1.split("::").last().unwrap()
    }
    pub fn full_fn_name(&self) -> &str {
        &self.1
    }
    pub fn file_name(&self) -> String {
        let split_str = self.1.split("::");
        let mut split_as_vec = split_str.collect::<Vec<&str>>();
        split_as_vec.pop();
        let new_str = split_as_vec.join("::");
        let new_str_cloned = new_str.clone();
        new_str_cloned
    }
    pub fn read_source<S: FileSystem>(&self, fs: &S) -> Result<String, S::FSError> {
        fs.read(&self.0)
    }

    pub fn write_source<S: FileSystem>(&self, fs: &S, str: &str) -> Result<(), S::FSError> {
        fs.write(&self.0, str.as_bytes())
    }
}

impl Display for Loc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.0.to_str().unwrap(), self.1)
    }
}

impl From<(RawLoc, string_cache::DefaultAtom)> for Loc {
    fn from((loc, name): (RawLoc, string_cache::DefaultAtom)) -> Self {
        match loc.filename {
            rustc_span::FileName::Real(path) => {
                Loc(path.local_path().unwrap().into(), format!("{}", name))
            }
            _ => panic!("unsupported source location"),
        }
    }
}
