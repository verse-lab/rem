use std::process::Command;

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
