mod projects;
mod utils;

use crate::projects::PATH_TO_EXPERIMENT_PROJECTS;
use crate::utils::{
    get_latest_commit, rename_callee, reset_to_base_branch, run_extraction, update_expr_branch,
    ExtractionResult, CALLEE_NAME,
};
use log::info;
use std::fs;

fn main() {
    env_logger::init();
    let result_n = fs::read_dir("./results").unwrap().count();
    let mut wtr = csv::Writer::from_path(format!("./results/result_{}.csv", result_n)).unwrap();
    for expr_project in projects::all() {
        let repo_path = format!("{}/{}", PATH_TO_EXPERIMENT_PROJECTS, expr_project.project);
        for experiment in expr_project.experiments {
            for i in 1..(experiment.extractions.len() + 1) {
                let expr_branch = format!("{}{}-expr", experiment.expr_type, i);
                let expr_branch_active = format!("{}{}-expr-active", experiment.expr_type, i);
                // rename_callee(&repo_path, &expr_branch, "bar", CALLEE_NAME, experiment.extractions.get(i - 1).unwrap());

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
                    total_duration_s: 0.,
                    commit: "".to_string(),
                    commit_url: "".to_string(),
                    failed_at: None,
                    project: expr_project.project.clone(),
                    branch: expr_branch_active.clone(),
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

                either!(
                    update_expr_branch(&repo_path, &expr_branch_active),
                    panic!("could not update experiment branch!")
                );

                extraction_result.commit = get_latest_commit(&repo_path);
                extraction_result.commit_url = format!(
                    "{}/commit/{}",
                    expr_project.project_url, extraction_result.commit
                );

                info!("experiment branch HEAD <--- {}", extraction_result.commit);

                wtr.serialize(extraction_result)
                    .expect("failed to write experiment results!");
            }
        }
    }
    wtr.flush().expect("failed to flush csv")
}
