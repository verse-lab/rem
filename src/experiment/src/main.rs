#![feature(variant_count)]

mod projects;
mod utils;

use crate::projects::PATH_TO_EXPERIMENT_PROJECTS;
use crate::utils::{
    get_caller_size, get_latest_commit, get_project_size, get_src_size, reset_to_base_branch,
    run_extraction, update_expr_branch, upload_csv, ExtractionResult, Secrets,
};
use log::{info, warn};
use std::fs;
use std::string::ToString;

const RESULT_SPREADSHEET: &str = "121Lwpv03Vq5K4IBdbQGn7OS5aBGPVKg-jDn8xczkXJc";
const RESULT_SHEET_ID: i32 = 549359316;

fn main() {
    env_logger::init();
    let secrets_content = fs::read_to_string("secrets.json").unwrap();
    let secrets = serde_json::from_str::<Secrets>(secrets_content.as_str()).unwrap();
    let result_n = fs::read_dir("./results").unwrap().count();
    let csv_file = format!("./results/result_{}.csv", result_n);
    let mut wtr = csv::Writer::from_path(&csv_file).unwrap();
    info!("Currently running {} experiments!", projects::size());
    for expr_project in projects::all() {
        let repo_path = format!("{}/{}", PATH_TO_EXPERIMENT_PROJECTS, expr_project.project);
        for experiment in expr_project.experiments {
            for i in 1..(experiment.extractions.len() + 1) {
                let extraction = experiment.extractions.get(i - 1).unwrap();
                let expr_branch = format!("{}{}-expr", experiment.expr_type, i);
                let expr_branch_active = format!("{}{}-expr-active", experiment.expr_type, i);
                // rename_callee(&repo_path, &expr_branch, "bar", CALLEE_NAME, experiment.extractions.get(i - 1).unwrap());

                // reset all branch to their base branch
                either!(
                    reset_to_base_branch(&repo_path, &expr_branch, &expr_branch_active),
                    panic!(
                        "could not reset to initial state for {}:{}",
                        &expr_project.project, &expr_branch
                    )
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
                    project_size: get_project_size(extraction),
                    src_size: get_src_size(extraction),
                    caller_size: get_caller_size(extraction),
                    features: String::new(),
                    notes: extraction.notes.clone(),
                };

                let (success, duration) = run_extraction(extraction, &mut extraction_result);
                info!(
                    "extraction completed success : {}, duration: {}",
                    success,
                    duration.as_secs()
                );

                either!(
                    update_expr_branch(&repo_path, &expr_branch_active),
                    panic!(
                        "could not update experiment branch for {}:{}!",
                        &expr_project.project, &expr_branch
                    )
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
    wtr.flush().expect("failed to flush csv");

    either!(
        upload_csv(
            &secrets,
            &csv_file,
            &RESULT_SPREADSHEET.to_string(),
            RESULT_SHEET_ID,
            0,
            0
        ),
        warn!(
            "failed to upload result csv! please upload {} manually.",
            csv_file
        )
    );
}
