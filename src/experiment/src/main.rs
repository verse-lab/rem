mod projects;
mod utils;

use crate::projects::PATH_TO_EXPERIMENT_PROJECTS;
use crate::utils::{
    get_latest_commit, reset_to_base_branch, run_extraction, update_expr_branch, ExtractionResult,
};
use log::info;
use walkdir::WalkDir;

fn main() {
    env_logger::init();
    let result_n = WalkDir::new("./results").into_iter().count();
    let mut wtr = csv::Writer::from_path(format!("./results/result_{}.csv", result_n)).unwrap();
    for expr_project in projects::all() {
        let repo_path = format!("{}/{}", PATH_TO_EXPERIMENT_PROJECTS, expr_project.project);
        for experiment in expr_project.experiments {
            for i in 1..(experiment.extractions.len() + 1) {
                let expr_branch = format!("{}{}-expr", experiment.expr_type, i);
                let expr_branch_active = format!("{}{}-expr-active", experiment.expr_type, i);

                // reset all branch to their base branch
                either!(
                    reset_to_base_branch(&repo_path, &expr_branch, &expr_branch_active),
                    panic!("could not reset to initial state")
                );

                info!(
                    "checked out: {} {}<--- HEAD: {}",
                    expr_project.project,
                    expr_branch_active,
                    get_latest_commit(&repo_path)
                );

                let mut extraction_result = ExtractionResult {
                    success: false,
                    fix_nlcf_duration_ms: Default::default(),
                    fix_borrow_duration_ms: Default::default(),
                    fix_lifetime_cargo_ms: Default::default(),
                    cargo_cycles: 0,
                    total_duration_ms: Default::default(),
                    total_duration_s: 0,
                    commit: "".to_string(),
                    commit_url: "".to_string(),
                    failed_at: None,
                };

                let (success, duration) = run_extraction(
                    experiment.extractions.get(i - 1).unwrap(),
                    &mut extraction_result,
                );
                info!(
                    "extraction completed success : {}, duration: {}",
                    success,
                    duration.as_secs()
                );

                wtr.serialize(extraction_result)
                    .expect("failed to write experiment results!");

                either!(
                    update_expr_branch(&repo_path, &expr_branch_active),
                    panic!("could not update experiment branch!")
                );
                info!(
                    "experiment branch HEAD <--- {}",
                    get_latest_commit(&repo_path)
                )
            }
        }
    }
    wtr.flush().expect("failed to flush csv")
}
