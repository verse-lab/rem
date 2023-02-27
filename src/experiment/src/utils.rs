use std::process::Command;

pub fn checkout(dir: String, branch: String) -> bool {
    let mut checkout = Command::new("git");
    checkout.arg("-C").arg(dir).arg("checkout").arg(branch);
    let out = checkout.output().unwrap();
    out.status.success()
}

pub fn checkout_b(dir: String, branch: String) -> bool {
    let mut checkout = Command::new("git");
    checkout
        .arg("-C")
        .arg(dir)
        .arg("checkout")
        .arg("-b")
        .arg(branch);
    let out = checkout.output().unwrap();
    out.status.success()
}
