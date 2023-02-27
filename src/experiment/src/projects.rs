pub struct Extraction {
    pub src_name: String,
    pub src_path: String,
    pub caller: String,
    pub cargo_path: String,
    pub original_path: String,
    pub mut_methods_path: String,
}

impl Extraction {
    fn new(src_path: &str, caller: &str, cargo_path: &str) -> Self {
        let src_name = match src_path.split("/").last() {
            None => panic!("invalid path maybe"),
            Some(tmp) => match tmp.strip_suffix(".rs") {
                None => panic!("invalid rust file"),
                Some(src_name) => src_name,
            },
        }
        .to_string();

        let original_path = format!("{}_ORIGINAL", src_path);
        let mut_methods_path = format!("{}_MUTABLE_METHOD_CALLS", src_path);

        Self {
            src_name,
            src_path: src_path.to_string(),
            caller: caller.to_string(),
            cargo_path: cargo_path.to_string(),
            original_path,
            mut_methods_path,
        }
    }
}

pub struct Experiment {
    pub expr_type: String,
    pub extractions: Vec<Extraction>,
}
pub struct ExperimentProject {
    pub project: String,
    pub experiments: Vec<Experiment>,
}

// ORIGINAL PATH is <SRC NAME>_ORIGINAL
// MUTABLE METHOD CALL is <SRC NAME>_MUTABLE_METHOD_CALLS
// CALLEE is always "bar"

pub fn all() -> Vec<ExperimentProject> {
    vec![gitoxide()]
}

/// gitoxide experiment
pub fn gitoxide() -> ExperimentProject {
    ExperimentProject {
        project: "gitoxide".to_string(),
        experiments: vec![
            Experiment {
                expr_type: "ext".to_string(),
                extractions: vec![
                    Extraction::new(
                        "gix-pack/src/verify.rs",
                        "checksum_on_disk_or_mmap",
                        "gix-pack/Cargo.toml",
                    ),
                    Extraction::new(
                        "gix-mailmap/src/parse.rs",
                        "parse_line",
                        "gix-mailmap/Cargo.toml",
                    ),
                ],
            },
            Experiment {
                expr_type: "ext-com".to_string(),
                extractions: vec![
                    Extraction::new(
                        "git-protocol/src/packet_line/decode.rs",
                        "streaming",
                        "git-protocol/Cargo.toml",
                    ),
                    Extraction::new(
                        "git-config/src/file/resolve_includes.rs",
                        "resolve_includes_recursive",
                        "git-config/Cargo.toml",
                    ),
                ],
            },
            Experiment {
                expr_type: "inline-ext".to_string(),
                extractions: vec![
                    Extraction::new(
                        "gix-validate/src/reference.rs",
                        "name",
                        "gix-validate/Cargo.toml",
                    ),
                    Extraction::new(
                        "gix-object/src/parse.rs",
                        "signature",
                        "gix-object/Cargo.toml",
                    ),
                    Extraction::new("gix/src/create.rs", "into", "gix/Cargo.toml"),
                    Extraction::new(
                        "gix-lock/src/acquire.rs",
                        "lock_with_mode",
                        "gix-lock/Cargo.toml",
                    ), // diff from above (different function extracted)
                    Extraction::new(
                        "gix-lock/src/acquire.rs",
                        "lock_with_mode",
                        "gix-lock/Cargo.toml",
                    ),
                    Extraction::new("gix-discover/src/is.rs", "git", "gix-discover/Cargo.toml"),
                    Extraction::new("gix-glob/src/parse.rs", "pattern", "gix-glob/Cargo.toml"),
                ],
            },
        ],
    }
}
