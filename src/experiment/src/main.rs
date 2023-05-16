#![feature(variant_count)]

mod projects;
mod utils;

use crate::projects::{PATH_TO_EXPERIMENT_PROJECTS};
use crate::utils::{get_caller_callee_size, get_latest_commit, get_project_size, get_src_size, reset_to_base_branch, run_extraction, update_expr_branch, upload_csv, ExtractionResult, Secrets, checkout, push_branch};
use log::{info, warn, debug};
use std::fs;
use std::path::Path;
use std::string::ToString;

use std::process::Command;

const RESULT_SPREADSHEET: &str = "121Lwpv03Vq5K4IBdbQGn7OS5aBGPVKg-jDn8xczkXJc";
const RESULT_SHEET_ID: i32 = 549359316;
const RUN_EXTRACTION: bool = false;
const CREATE_ARTEFACTS: bool = true;
const CARGO_CLEAN: bool = true;

fn main() {
    env_logger::init();
    let secrets_content = fs::read_to_string("secrets.json").unwrap();
    let secrets = serde_json::from_str::<Secrets>(secrets_content.as_str()).unwrap();
    let result_n = fs::read_dir("./results").unwrap().count();
    let csv_file = format!("./results/result_{}.csv", result_n);
    let mut wtr = csv::Writer::from_path(&csv_file).unwrap();
    info!("Currently running {} experiments!", projects::size());
    for expr_project in projects::all() {
        let repo_path = format!("{}/{}", PATH_TO_EXPERIMENT_PROJECTS, &expr_project.project);
        for experiment in expr_project.experiments {
            for i in 1..(experiment.extractions.len() + 1) {
                let arte_fixed = format!("../../../artefact_sample_projects/{}-{}{}", &expr_project.project, experiment.expr_type, i);
                debug!("running for {}", arte_fixed);
                let extraction = experiment.extractions.get(i - 1).unwrap();
                let expr_branch = format!("{}{}-expr", experiment.expr_type, i);
                let expr_branch_active = format!("{}{}-expr-active", experiment.expr_type, i);

                if CREATE_ARTEFACTS {
                    let expr_arte = format!("{}{}-expr-ra", experiment.expr_type, i);
                    let arte_clone = format!("../../../artefact_sample_projects/{}", &expr_project.project);
                    if !(Path::new(&arte_clone).is_dir()) {
                        let mut cmd = Command::new("gix");
                        cmd.arg("clone").arg(&expr_project.project_url).arg(&arte_clone);
                        let out = cmd.output().unwrap();
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        debug!("cloned: {}, {}", out.status.success(), stderr);
                    }

                    if !((utils::stash(&arte_clone) || true) // don't care about stashing
                        && utils::checkout(&arte_clone, &expr_arte)) {
                        let expr_arte_tmp = format!("{}{}-expr-ij", experiment.expr_type, i);
                        assert!(reset_to_base_branch(&arte_clone, &expr_arte_tmp, &expr_arte));
                    };

                    if !(Path::new(&arte_fixed).is_dir()) {
                        let mut cmd = Command::new("cp");
                        cmd.arg("-r").arg(&arte_clone).arg(&arte_fixed);
                        let out = cmd.output().unwrap();
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        debug!("cp: {}, {}", out.status.success(), stderr);
                    }

                    // change remote url to ssh so can push
                    let mut cmd = Command::new("git");
                    cmd.arg("-C").arg(&arte_fixed).arg("remote").arg("set-url").arg("origin").arg(format!("git@github.com:sewenthy/{}.git", &expr_project.project).as_str());
                    let out = cmd.output().unwrap();
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    debug!("remote re-url: {}, {}", out.status.success(), stderr,);

                    // checkout to new branch for artefact
                    let arte_branch = format!("{}{}-expr-artefact", &experiment.expr_type, i);
                    // checkout_b(&arte_fixed, &arte_branch);
                    // push new branch
                    push_branch(&arte_fixed, &arte_branch, true);
                    // run cargo clean
                    if CARGO_CLEAN {
                        let mut cmd = Command::new("cargo");
                        let toml = format!("--manifest-path={}/{}", &arte_fixed, extraction.cargo_path.replace(&extraction.project_path, ""));
                        cmd.arg("clean").arg(toml);
                        let out = cmd.output().unwrap();
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        debug!("cargo cealn: {}, {}", out.status.success(), stderr);
                    }
                    continue;
                }

                // rename_callee(&repo_path, &expr_branch, "bar", CALLEE_NAME, experiment.extractions.get(i - 1).unwrap());

                // reset all branch to their base branch
                if RUN_EXTRACTION {
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
                } else {
                    checkout(&repo_path, &expr_branch);
                }

                let (caller_size, callee_size) = get_caller_callee_size(extraction);

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
                    caller_size,
                    callee_size,
                    num_inputs: 0,
                    features: String::new(),
                    features_inner: vec![],
                    intellij_rust_old: extraction.intellij_old_rust,
                    rust_analyzer: extraction.rust_analyzer,
                    notes: extraction.notes.clone(),
                };

                if !RUN_EXTRACTION {
                    info!("project {}, {}, has src_size: {}, and caller_size: {}",
                        expr_project.project, expr_branch_active, extraction_result.src_size, extraction_result.caller_size,
                    );
                    if extraction_result.callee_size > extraction_result.caller_size {
                        panic!("weird!!");
                    }
                    wtr.serialize(extraction_result)
                        .expect("failed to write experiment results!");
                    continue
                }

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
    if RUN_EXTRACTION {
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
}
