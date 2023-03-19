use crate::either;
use std::path::Path;

pub const PATH_TO_EXPERIMENT_PROJECTS: &str = "/home/sewen/class/Capstone/sample_projects";

pub struct Extraction {
    pub src_name: String,
    pub src_path: String,
    pub caller: String,
    pub cargo_path: String,
    pub original_path: String,
    pub mut_methods_path: String,
    pub notes: Option<String>,
}

impl Extraction {
    fn new(project_path: &String, src_path: &str, caller: &str, cargo_path: &str, notes: Option<&str>) -> Self {
        let src_name = match src_path.split("/").last() {
            None => panic!("invalid path maybe"),
            Some(tmp) => match tmp.strip_suffix(".rs") {
                None => panic!("invalid rust file"),
                Some(src_name) => src_name,
            },
        }
        .to_string();

        let src_path = format!("{}/{}", project_path, src_path);

        let original_path = format!("{}_ORIGINAL", src_path);
        let mut_methods_path = format!("{}_MUTABLE_METHOD_CALLS", src_path);
        let cargo_path = format!("{}/{}", project_path, cargo_path);

        Self {
            src_name,
            src_path,
            caller: caller.to_string(),
            cargo_path,
            original_path,
            mut_methods_path,
            notes: notes.map(|s| s.to_string()),
        }
    }

    pub fn validate_paths(&self) {
        let paths = vec![
            self.src_path.as_str(),
            self.original_path.as_str(),
            self.mut_methods_path.as_str(),
            self.cargo_path.as_str(),
        ];
        paths.iter().for_each(|path| {
            either!(
                Path::new(path).exists(),
                panic!("{} does not exists!", path)
            )
        });
    }
}

pub struct Experiment {
    pub expr_type: String,
    pub extractions: Vec<Extraction>,
}
pub struct ExperimentProject {
    pub project: String,
    pub project_url: String,
    pub experiments: Vec<Experiment>,
}

// ORIGINAL PATH is <SRC NAME>_ORIGINAL
// MUTABLE METHOD CALL is <SRC NAME>_MUTABLE_METHOD_CALLS

pub fn all() -> Vec<ExperimentProject> {
    vec![petgraph(), gitoxide(), kickoff(), sniffnet(), beerus()]
}

/// gitoxide experiment
pub fn gitoxide() -> ExperimentProject {
    let project = "gitoxide".to_string();
    let project_url = "https://github.com/sewenthy/gitoxide".to_string();
    let project_path = format!("{}/{}", PATH_TO_EXPERIMENT_PROJECTS, project);

    ExperimentProject {
        project,
        project_url,
        experiments: vec![
            Experiment {
                expr_type: "ext".to_string(),
                extractions: vec![
                    Extraction::new(
                        &project_path,
                        "gix-pack/src/verify.rs",
                        "fan",
                        "gix-pack/Cargo.toml",
                        None,
                    ),
                    Extraction::new(
                        &project_path,
                        "gix-mailmap/src/parse.rs",
                        "parse_line",
                        "gix-mailmap/Cargo.toml",
                        Some("complex lifetime + bounds + nlcf--used in paper")
                    ),
                    Extraction::new(
                        &project_path,
                        "gix-hash/src/object_id.rs",
                        "from_hex",
                        "gix-hash/Cargo.toml",
                        Some("extracted within impl + invoc Self::bar")
                    ),
                ],
            },
            Experiment {
                expr_type: "ext-com".to_string(),
                extractions: vec![
                    Extraction::new(
                        &project_path,
                        "git-protocol/src/packet_line/decode.rs",
                        "streaming",
                        "git-protocol/Cargo.toml",
                        Some("nclf")
                    ),
                    Extraction::new(
                        &project_path,
                        "git-config/src/file/resolve_includes.rs",
                        "resolve_includes_recursive",
                        "git-config/Cargo.toml",
                        Some("2 lifetimes usage + good elision")
                    ),
                ],
            },
            Experiment {
                expr_type: "inline-ext".to_string(),
                extractions: vec![
                    Extraction::new(
                        &project_path,
                        "gix-validate/src/reference.rs",
                        "name",
                        "gix-validate/Cargo.toml",
                        Some("nclf + lifetime within traits + some non-elidibles")
                    ),
                    Extraction::new(
                        &project_path,
                        "gix-object/src/parse.rs",
                        "signature",
                        "gix-object/Cargo.toml",
                        Some("generic has lifetimes + very complex boundings--good to show")
                    ),
                    Extraction::new(&project_path, "gix/src/create.rs", "into", "gix/Cargo.toml", Some("failed due to cargo check")),
                    Extraction::new(
                        &project_path,
                        "gix-lock/src/acquire.rs",
                        "lock_with_mode",
                        "gix-lock/Cargo.toml",
                        None,
                    ), // diff from above (different function extracted)
                    Extraction::new(
                        &project_path,
                        "gix-lock/src/acquire.rs",
                        "lock_with_mode",
                        "gix-lock/Cargo.toml",
                        None,
                    ),
                    Extraction::new(
                        &project_path,
                        "gix-discover/src/is.rs",
                        "git",
                        "gix-discover/Cargo.toml",
                        None,
                    ),
                    Extraction::new(
                        &project_path,
                        "gix-glob/src/parse.rs",
                        "pattern",
                        "gix-glob/Cargo.toml",
                        None,
                    ),
                ],
            },
        ],
    }
}

/// sniffnet experiment
pub fn sniffnet() -> ExperimentProject {
    let project = "sniffnet".to_string();
    let project_url = "https://github.com/sewenthy/sniffnet".to_string();
    let project_path = format!("{}/{}", PATH_TO_EXPERIMENT_PROJECTS, project);

    ExperimentProject {
        project,
        project_url,
        experiments: vec![Experiment {
            expr_type: "inline-ext".to_string(),
            extractions: vec![
                Extraction::new(
                    &project_path,
                    "src/utility/manage_packets.rs",
                    "modify_or_insert_in_map",
                    "Cargo.toml",
                    Some("all elidible lifetimes")
                ),
                Extraction::new(
                    &project_path,
                    "src/thread_parse_packets.rs",
                    "parse_packets_loop",
                    "Cargo.toml",
                    Some("technial; need to introduce A{x=*x} if taken x as reference and init struct")
                ),
            ],
        }],
    }
}

/// kickoff experiment
pub fn kickoff() -> ExperimentProject {
    let project = "kickoff".to_string();
    let project_url = "https://github.com/sewenthy/kickoff".to_string();
    let project_path = format!("{}/{}", PATH_TO_EXPERIMENT_PROJECTS, project);

    ExperimentProject {
        project,
        project_url,
        experiments: vec![Experiment {
            expr_type: "inline-ext".to_string(),
            extractions: vec![Extraction::new(
                &project_path,
                "src/gui.rs",
                "register_inputs",
                "Cargo.toml",
                Some("all elidible lifetimes")
            )],
        }],
    }
}

/// beerus experiment
pub fn beerus() -> ExperimentProject {
    let project = "beerus".to_string();
    let project_url = "https://github.com/sewenthy/beerus".to_string();
    let project_path = format!("{}/{}", PATH_TO_EXPERIMENT_PROJECTS, project);

    ExperimentProject {
        project,
        project_url,
        experiments: vec![Experiment {
            expr_type: "ext-com".to_string(),
            extractions: vec![Extraction::new(
                &project_path,
                "beerus_rest_api/src/main.rs",
                "rocket",
                "beerus_rest_api/Cargo.toml",
                Some("small use of async")
            )],
        }],
    }
}

pub fn petgraph() -> ExperimentProject {
    let project = "petgraph".to_string();
    let project_url = "https://github.com/sewenthy/petgraph".to_string();
    let project_path = format!("{}/{}", PATH_TO_EXPERIMENT_PROJECTS, project);

    ExperimentProject {
        project,
        project_url,
        experiments: vec![Experiment {
            expr_type: "ext".to_string(),
            extractions: vec![
                Extraction::new(&project_path, "src/generate.rs", "all", "Cargo.toml", Some("within impl")),
                Extraction::new(&project_path, "src/graphmap.rs", "next", "Cargo.toml", Some("new impl with generics annotated + invoc using self.bar")),
            ],
        }],
    }
}

#[allow(dead_code)]
/// demo for testing only
pub fn demo() -> ExperimentProject {
    let project = "demo".to_string();
    let project_url = "https://github.com/sewenthy/capstone-demo".to_string();
    let project_path = format!("{}/{}", PATH_TO_EXPERIMENT_PROJECTS, project);

    ExperimentProject {
        project,
        project_url,
        experiments: vec![Experiment {
            expr_type: "ext".to_string(),
            extractions: vec![
                Extraction::new(&project_path, "src/main.rs", "trait_function", "Cargo.toml", None),
                Extraction::new(&project_path, "src/main.rs", "test", "Cargo.toml", None),
            ],
        }],
    }
}
