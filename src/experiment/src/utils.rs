use std::process::Command;
use std::time::{Duration, SystemTime};

use borrower::borrow::make_borrows;
use controller::non_local_controller::make_controls;
use repairer::repair_lifetime_loosest_bound_first::Repairer;
use crate::projects::Extraction;
/******************************* GIT RELATED  ***************************************************/
pub fn checkout(dir: &String, branch: &String) -> bool {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).arg("checkout").arg(branch);
    let out = cmd.output().unwrap();
    out.status.success()
}

pub fn checkout_b(dir: &String, branch: &String) -> bool {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).arg("checkout").arg("-b").arg(branch);
    let out = cmd.output().unwrap();
    out.status.success()
}

pub fn del_branch(dir: &String, branch: &String) -> bool {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).arg("branch").arg("-D").arg(branch);
    let out = cmd.output().unwrap();
    out.status.success()
}

pub fn push_branch(dir: &String, branch: &String) -> bool {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(dir)
        .arg("push")
        .arg("-u")
        .arg("fork") // always push to fork
        .arg(branch)
        .arg("--force");
    let out = cmd.output().unwrap();
    out.status.success()
}

pub fn commit(dir: &String, message: String) -> bool {
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
        panic!()
    }
}

pub fn reset_to_base_branch(dir: &String, base_branch: &String, active_branch: &String) -> bool {
    del_branch(dir, active_branch) && checkout(dir, base_branch) && checkout_b(dir, active_branch)
}

/*************************************** Extraction Related ************************************/
pub fn time_exec(f: &dyn Fn() -> bool) -> (bool, Duration) {
    let now = SystemTime::now();
    let success = f();
    let time_elapsed = now.elapsed().unwrap();
    (success, time_elapsed)
}

pub fn run_controller(extraction : Extraction) -> (bool, Duration) {
    let f = || make_controls(extraction.src_path.as_str(), extraction.src_path.as_str(), "bar", extraction.caller.as_str());
    time_exec(&f)
}

pub fn run_borrower(extraction: Extraction) -> (bool, Duration) {
    let f = || make_borrows(extraction.src_path.as_str(), extraction.src_path.as_str(), extraction.mut_methods_path.as_str(), "bar", extraction.caller.as_str(), extraction.original_path.as_str());
    time_exec(&f)
}

pub fn run_repairer(extraction: Extraction)-> (bool, Duration) {
    let mut repairer = repairer::repair_lifetime_loosest_bound_first::Repairer {};
    let f = || repairer.repair_project(extraction.src_path.as_str(), extraction.cargo_path, "bar");
    time_exec(&f)
}