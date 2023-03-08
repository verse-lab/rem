use std::fs;
use log::{debug, info, warn};
use std::ops::Add;
use std::process::Command;
use std::time::{Duration, SystemTime};
use regex::Regex;

use crate::projects::Extraction;
use borrower::borrow::make_borrows;
use controller::non_local_controller::make_controls;
use repairer::common::RepairSystem;
use repairer::repair_lifetime_loosest_bound_first::Repairer;
use utils::{check_project, find_caller};

pub const CALLEE_NAME: &str = "bar____EXTRACT_THIS";

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

pub fn update_expr_branch(dir: &String, active_branch: &String) -> bool {
    commit(dir, active_branch) && push_branch(dir, active_branch, true)
}

#[allow(dead_code)]
pub fn rename_callee(
    dir: &String,
    branch: &String,
    callee_old_name: &str,
    callee_name: &str,
    e: &Extraction,
) {
    either!(
        checkout(dir, branch),
        panic!("could not check out branch {} at {}", branch, dir)
    );
    let replace = format!("s/{}(/{}(/g", callee_old_name, callee_name);
    let mut cmd = Command::new("sed");
    cmd.arg("-i").arg(replace).arg(&e.src_path);
    let _out = cmd.output().unwrap();
    let replace2 = format!("s/{}</{}</g", callee_old_name, callee_name);
    let mut cmd2 = Command::new("sed");
    cmd2.arg("-i").arg(replace2).arg(&e.src_path);
    let out = cmd.output().unwrap();
    either!(
        out.status.success(),
        warn!(
            "could not rename callee turbofish: {}",
            String::from_utf8_lossy(&out.stderr)
        )
    );
    let msg = format!("{} renamed callee to {}", branch, callee_name);
    if !update_expr_branch(dir, &msg) {
        warn!("failed to rename callee")
    }
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
    pub project_size: i32,
    pub src_size: i32,
    pub caller_size: i32,
}

pub fn read_cargo_count(stats: &str) -> i32 {
    let re = Regex::new(r"Rust\D+\d+\D+\d+\D+\d+\D+\d+\D+(?P<code_size>\d+)")
        .unwrap();
    match re.captures(stats.as_ref()) {
        Some (captured) => match captured["code_size"].parse::<i32>() {
            Ok(n) => n,
            Err(_) => panic!("did not find code size"),
        },
        None => panic!("did not match rust code"),
    }
}

pub fn get_project_size(e: &Extraction) -> i32 {
    let mut cmd = Command::new("cargo");
    let path = e.cargo_path.clone().replace("Cargo.toml", "");
    cmd.arg("count").arg(&path).arg("--exclude").arg(format!("{}.git,{}*/test.rs,{}target", &path, &path, &path)).arg("-l").arg("rs");
    let out = cmd.output().unwrap();
    if out.status.success() {
        let stats = String::from_utf8_lossy(&out.stdout);
        debug!("found stats: {}", stats.as_ref());
        read_cargo_count(stats.as_ref())
    } else {
        panic!("no commit hash found for HEAD")
    }
}

pub fn get_src_size(e: &Extraction) -> i32 {
    let mut cmd = Command::new("cargo");
    cmd.arg("count").arg(&e.src_path);
    let out = cmd.output().unwrap();
    if out.status.success() {
        let stats = String::from_utf8_lossy(&out.stdout);
        debug!("found stats: {}", stats.as_ref());
        read_cargo_count(stats.as_ref())
    } else {
        panic!("no commit hash found for HEAD")
    }
}

pub fn get_caller_size(e: &Extraction) -> i32 {
    let (found, caller, callee) = find_caller(e.src_path.as_str(), e.caller.as_str(), CALLEE_NAME);
    either!(found, panic!("did not find caller/callee"));
    let path = "/tmp/some_caller_tmp.rs";
    fs::write(path, format!("{}\n\n{}", caller, callee)).unwrap();
    let mut cmd = Command::new("cargo");
    cmd.arg("count").arg(path);
    let out = cmd.output().unwrap();
    if out.status.success() {
        let stats = String::from_utf8_lossy(&out.stdout);
        debug!("found stats: {}", stats.as_ref());
        read_cargo_count(stats.as_ref())
    } else {
        panic!("no commit hash found for HEAD")
    }
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
            CALLEE_NAME,
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
            CALLEE_NAME,
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
            CALLEE_NAME,
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
