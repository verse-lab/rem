mod projects;
mod utils;

use crate::utils::{
    get_latest_commit, reset_to_base_branch, run_extraction,
};
use log::info;

const PATH_TO_EXPERIMENT_PROJECTS: &str = "/home/sewen/class/Capstone/sample_projects/";

fn main() {
    env_logger::init();
    for expr_project in projects::all() {
        for experiment in expr_project.experiments {
            for i in 1..(experiment.extractions.len() + 1) {
                let repo_path = format!("{}/{}", PATH_TO_EXPERIMENT_PROJECTS, expr_project.project);
                let expr_branch = format!("{}{}-expr", experiment.expr_type, i);
                let expr_branch_active = format!("{}{}-expr-active", experiment.expr_type, i);

                // reset all branch to their base branch
                reset_to_base_branch(&repo_path, &expr_branch, &expr_branch_active)
                    || panic!("could not reset to initial state");

                info!(
                    "checked out: {} {}<--- HEAD: {}",
                    expr_project.project,
                    expr_branch_active,
                    get_latest_commit(&repo_path)
                );
                let (success, duration) = run_extraction(experiment.extractions.get(i).unwrap());
                info!("extraction completed success : {}, duration: {}", success, duration.as_secs())
            }
        }
    }
}
