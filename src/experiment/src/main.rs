mod utils;

use crate::utils::{checkout_b, del_branch, get_latest_commit};
use log::info;
use utils::checkout;

const PATH_TO_EXPERIMENT_PROJECTS: &str = "/home/sewen/class/Capstone/sample_projects/";

struct Extraction {
    src_name: String,
    src_path: String,
    caller: String,
    cargo_path: String,
    original_path: String,
    mut_methods_path: String,
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

struct Experiment {
    expr_type: String,
    count: i32,
    extractions: Vec<Extraction>,
}
struct ExperimentProject {
    project: String,
    experiments: Vec<Experiment>,
}

fn main() {
    env_logger::init();
    // ORIGINAL PATH is <SRC NAME>_ORIGINAL
    // MUTABLE METHOD CALL is <SRC NAME>_MUTABLE_METHOD_CALLS
    // CALLEE is always "bar"
    let projects = vec![ExperimentProject {
        project: "gitoxide".to_string(),
        experiments: vec![
            Experiment {
                expr_type: "ext".to_string(),
                count: 2,
                extractions: vec![
                    Extraction::new("gix-discover/src/is.rs", "git", "gix-discover/Cargo.toml"),
                    Extraction::new(
                        "gix-mailmap/src/parse.rs",
                        "parse_line",
                        "gix-mailmap/Cargo.toml",
                    ),
                ],
            },
            Experiment {
                expr_type: "ext-com".to_string(),
                count: 2,
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
                count: 6,
                extractions: vec![],
            },
        ],
    }];
    for expr_project in projects {
        for experiment in expr_project.experiments {
            for i in 1..(experiment.count + 1) {
                let repo_path = format!("{}/{}", PATH_TO_EXPERIMENT_PROJECTS, expr_project.project);
                let expr_branch = format!("{}{}-expr", experiment.expr_type, i);
                let expr_branch_active = format!("{}{}-expr-active", experiment.expr_type, i);

                // reset all branch to their base branch
                let _ = del_branch(&repo_path, &expr_branch_active);
                let _ = checkout(&repo_path, &expr_branch);
                let _ = checkout_b(&repo_path, &expr_branch_active);
                info!(
                    "checked out: {} {}<--- HEAD: {}",
                    expr_project.project,
                    expr_branch_active,
                    get_latest_commit(&repo_path)
                );
            }
        }
    }
}
