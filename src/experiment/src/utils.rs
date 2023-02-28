use log::{debug, info};
use std::ops::Add;
use std::process::Command;
use std::time::{Duration, SystemTime};

use crate::projects::Extraction;
use borrower::borrow::make_borrows;
use controller::non_local_controller::make_controls;
use repairer::common::RepairSystem;
use repairer::repair_lifetime_loosest_bound_first::Repairer;
use utils::check_project;

/*********************************    MISC    ***************************************************/
#[macro_export]
macro_rules! either {
    // macth like arm for macro
    ($a:expr,$b:expr) => {
        // macro expand to this code
        {
            // $a and $b will be templated using the value/variable provided to macro
            if !$a {
                $b
            }
        }
    };
}

/******************************* GIT RELATED  ***************************************************/
pub fn checkout(dir: &String, branch: &String) -> bool {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).arg("checkout").arg(branch);
    let out = cmd.output().unwrap();
    debug!(
        "checkout {}: {}, {}",
        branch,
        out.status.success(),
        String::from_utf8_lossy(&out.stderr)
    );
    out.status.success()
}

pub fn checkout_b(dir: &String, branch: &String) -> bool {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).arg("checkout").arg("-b").arg(branch);
    let out = cmd.output().unwrap();
    debug!(
        "make {}: {}, {}",
        branch,
        out.status.success(),
        String::from_utf8_lossy(&out.stderr)
    );
    out.status.success()
}

pub fn del_branch(dir: &String, branch: &String) -> bool {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).arg("branch").arg("-D").arg(branch);
    let out = cmd.output().unwrap();
    debug!(
        "deleted {}: {}, {:?}",
        branch,
        out.status.success(),
        String::from_utf8_lossy(&out.stderr)
    );
    out.status.success()
}

pub fn push_branch(dir: &String, branch: &String, force: bool) -> bool {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(dir)
        .arg("push")
        .arg("-u")
        .arg("fork") // always push to fork
        .arg(branch);
    if force {
        cmd.arg("--force");
    }
    let out = cmd.output().unwrap();
    out.status.success()
}

pub fn commit(dir: &String, message: &String) -> bool {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).arg("commit").arg("-am").arg(message);
    let out = cmd.output().unwrap();
    out.status.success()
}

pub fn get_latest_commit(dir: &String) -> String {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).arg("rev-parse").arg("HEAD");
    let out = cmd.output().unwrap();
    if out.status.success() {
        let hash = String::from_utf8_lossy(&out.stdout);
        hash.to_string()
    } else {
        panic!("no commit hash found for HEAD")
    }
}

pub fn reset_to_base_branch(dir: &String, base_branch: &String, active_branch: &String) -> bool {
    checkout(dir, base_branch)
        && del_branch(dir, active_branch)
        && checkout(dir, base_branch)
        && checkout_b(dir, active_branch)
}

/*************************************** Extraction Related ************************************/
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct ExtractionResult {
    pub success: bool,
    pub fix_nlcf_duration_ms: u128,
    pub fix_borrow_duration_ms: u128,
    pub fix_lifetime_cargo_ms: u128,
    pub cargo_cycles: i32,
    pub total_duration_ms: u128,
    pub total_duration_s: f64,
    pub commit: String,
    pub commit_url: String,
    pub failed_at: Option<String>,
    pub project: String,
    pub branch: String,
}

pub fn time_exec(name: &str, f: &mut dyn FnMut() -> bool) -> (bool, Duration) {
    let now = SystemTime::now();
    let success = f();
    let time_elapsed = now.elapsed().unwrap();
    info!(
        "{} {} in {}s",
        name,
        if success { "succeeded" } else { "failed" },
        time_elapsed.as_secs()
    );
    (success, time_elapsed)
}

pub fn run_controller(
    extraction: &Extraction,
    extraction_result: &mut ExtractionResult,
) -> (bool, Duration) {
    let mut f = || {
        make_controls(
            extraction.src_path.as_str(),
            extraction.src_path.as_str(),
            "bar",
            extraction.caller.as_str(),
        )
    };
    let (success, duration) = time_exec("controller", &mut f);
    either!(
        success,
        extraction_result.failed_at = Some("controller".to_string())
    );
    extraction_result.fix_nlcf_duration_ms = duration.as_millis();
    (success, duration)
}

pub fn run_borrower(
    extraction: &Extraction,
    extraction_result: &mut ExtractionResult,
) -> (bool, Duration) {
    let mut f = || {
        make_borrows(
            extraction.src_path.as_str(),
            extraction.src_path.as_str(),
            extraction.mut_methods_path.as_str(),
            "bar",
            extraction.caller.as_str(),
            extraction.original_path.as_str(),
        )
    };
    let (success, duration) = time_exec("borrower", &mut f);
    either!(
        success,
        extraction_result.failed_at = Some("borrower".to_string())
    );
    extraction_result.fix_borrow_duration_ms = duration.as_millis();
    (success, duration)
}

pub fn run_repairer(
    extraction: &Extraction,
    extraction_result: &mut ExtractionResult,
) -> (bool, Duration) {
    let repairer = Repairer {};
    let mut f = || {
        let (success, count) = repairer.repair_project(
            extraction.src_path.as_str(),
            extraction.cargo_path.as_str(),
            "bar",
        );
        debug!("cargo repair counted: {}", count);
        extraction_result.cargo_cycles = count;
        success
    };
    let (success, duration) = time_exec("cargo", &mut f);
    either!(
        success,
        extraction_result.failed_at = Some("cargo".to_string())
    );
    extraction_result.fix_lifetime_cargo_ms = duration.as_millis();
    (success, duration)
}

pub fn run_extraction(
    extraction: &Extraction,
    extraction_result: &mut ExtractionResult,
) -> (bool, Duration) {
    extraction.validate_paths();

    let mut first_check = || {
        check_project(&extraction.cargo_path, &vec![])
            .output()
            .unwrap()
            .status
            .success()
    };
    time_exec("first_check", &mut first_check);

    let actions: Vec<&dyn Fn(&Extraction, &mut ExtractionResult) -> (bool, Duration)> =
        vec![&run_controller, &run_borrower, &run_repairer];
    let (success, duration) = actions.iter().fold(
        (true, Duration::from_secs(0)),
        |(success, duration), &action| {
            if success {
                let (action_success, action_duration) = action(extraction, extraction_result);
                (action_success && success, duration.add(action_duration))
            } else {
                (success, duration)
            }
        },
    );
    extraction_result.success = success;
    extraction_result.total_duration_ms = duration.as_millis();
    extraction_result.total_duration_s = duration.as_millis() as f64 * 0.001;
    (success, duration)
}

pub fn update_expr_branch(dir: &String, active_branch: &String) -> bool {
    commit(dir, active_branch) && push_branch(dir, active_branch, true)
}
