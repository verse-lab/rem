use jwt_simple::prelude::*;
use log::{debug, info, warn};
use regex::Regex;
use reqwest::blocking::Client;
use std::fs;

use std::ops::Add;
use std::process::Command;
use std::time::{Duration, SystemTime};

use crate::projects::{Extraction, ExtractionResultOld};
use crate::utils::ExtractionFeature::{
    ImmutableBorrow, MutableBorrow, NonElidibleLifetimes, NonLocalLoop, NonLocalReturn,
    StructHasLifetimeSlot,
};
use rem_borrower::borrow::inner_make_borrows;
use rem_controller::non_local_controller::inner_make_controls;
use rem_repairer::common::RepairSystem;
use rem_repairer::repair_lifetime_loosest_bound_first::Repairer;
use rem_utils::{check_project, find_caller, format_source};

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

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridCoordinate {
    sheet_id: i32,
    row_index: i32,
    column_index: i32,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteDataRequest {
    coordinate: GridCoordinate,
    data: String,
    type_: String,
    delimiter: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteDataRequestWrapper {
    paste_data: PasteDataRequest,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpreadsheetsBatchUpdate {
    include_spreadsheet_in_response: bool,
    requests: Vec<PasteDataRequestWrapper>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Secrets {
    iss: String,
    api_key: String,
    private_key: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct JwtClaims {
    iss: String,
    scope: String,
    aud: String,
}

pub fn make_jwt(secrets: &Secrets) -> String {
    let key = RS256KeyPair::from_pem(secrets.private_key.as_str()).unwrap();

    let claims = JwtClaims {
        iss: secrets.iss.clone(),
        scope: "https://www.googleapis.com/auth/spreadsheets https://www.googleapis.com/auth/drive https://www.googleapis.com/auth/drive.file".to_string(),
        aud: "https://oauth2.googleapis.com/token".to_string(),
    };
    let claims = Claims::with_custom_claims(claims, Duration::from_secs(30).into());

    let token = key.sign(claims).unwrap();
    debug!("token: {}", token.as_str());
    token
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AccessTokenReq {
    grant_type: String,
    assertion: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AccessTokenRes {
    access_token: String,
}

pub fn get_gcp_access_token(secrets: &Secrets, client: &Client) -> String {
    let token = make_jwt(secrets);
    let body = AccessTokenReq {
        grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer".to_string(),
        assertion: token.as_str().to_string(),
    };

    match client
        .post("https://oauth2.googleapis.com/token")
        .form(&body)
        .send()
    {
        Ok(res) => {
            if res.status().is_success() {
                let res_json = res.json::<AccessTokenRes>().unwrap();
                res_json.access_token
            } else {
                panic!("failed to get access token: {:?}", res);
            }
        }
        Err(err) => {
            panic!("failed to get access token: {:?}", err);
        }
    }
}

pub fn upload_csv(
    secrets: &Secrets,
    csv_file: &String,
    spreadsheet: &String,
    sheet_id: i32,
    row_index: i32,
    column_index: i32,
) -> bool {
    let client = Client::new();

    let access_token = get_gcp_access_token(secrets, &client);

    let data: String = fs::read_to_string(csv_file).unwrap().parse().unwrap();
    let paste_data = PasteDataRequest {
        coordinate: GridCoordinate {
            sheet_id,
            row_index,
            column_index,
        },
        data,
        type_: "PASTE_NORMAL".to_string(),
        delimiter: ",".to_string(),
    };

    let body = SpreadsheetsBatchUpdate {
        include_spreadsheet_in_response: false,
        requests: vec![PasteDataRequestWrapper { paste_data }],
    };

    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}:batchUpdate?key={}",
        spreadsheet, secrets.api_key
    );
    let req = client.post(url).json(&body).bearer_auth(&access_token);
    debug!("about to send: {:?}", req);
    match req.send() {
        Ok(ok) => {
            if ok.status().is_success() {
                debug!("ok updated csv in sheet: {:?}", ok);
                true
            } else {
                warn!("failed to update csv status {:?}: {:?}", ok.status(), &ok);
                let text = ok.text().unwrap();
                warn!("body: {}", text);
                false
            }
        }
        Err(err) => {
            warn!("error updating csv: {:?}", err);
            false
        }
    }
}

/******************************* GIT RELATED  ***************************************************/
pub fn stash(dir: &String) -> bool {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).arg("stash");
    let out = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    debug!("stashed: {}, {}", out.status.success(), stderr,);
    out.status.success() || {
        warn!("stash failed! {}", stderr);
        true
    }
}

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
    (stash(dir) || true) // don't care about stashing
        && checkout(dir, base_branch)
        && (push_branch(dir, base_branch, false) || true) // don't care if can't push
        && (del_branch(dir, active_branch) || true) // don't care if can't delete
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
#[serde(rename_all = "snake_case")]
pub enum ExtractionFeature {
    NonLocalReturn,
    NonLocalLoop,
    ImmutableBorrow,
    MutableBorrow,
    StructHasLifetimeSlot,
    NonElidibleLifetimes,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct ExtractionResult {
    pub success: bool,
    pub project: String,
    pub branch: String,
    pub fix_nlcf_duration_ms: u128,
    pub fix_borrow_duration_ms: u128,
    pub fix_lifetime_cargo_ms: u128,
    pub cargo_cycles: i32,
    pub total_duration_ms: u128,
    pub total_duration_s: f64,
    pub commit: String,
    pub commit_url: String,
    pub failed_at: Option<String>,
    pub project_size: i32,
    pub src_size: i32,
    pub caller_size: i32,
    pub callee_size: i32,
    pub num_inputs: usize,
    pub features: String,
    #[serde(skip_serializing)]
    pub features_inner: Vec<ExtractionFeature>,
    pub intellij_rust_old: ExtractionResultOld,
    pub rust_analyzer: ExtractionResultOld,
    pub notes: Option<String>,
}

pub fn read_cargo_count(stats: &str) -> i32 {
    let re = Regex::new(r"Rust\D+\d+\D+\d+\D+\d+\D+\d+\D+(?P<code_size>\d+)").unwrap();
    match re.captures(stats.as_ref()) {
        Some(captured) => match captured["code_size"].parse::<i32>() {
            Ok(n) => n,
            Err(_) => panic!("did not find code size"),
        },
        None => panic!("did not match rust code"),
    }
}

pub fn get_project_size(e: &Extraction) -> i32 {
    let mut cmd = Command::new("cargo");
    let path = e.cargo_path.clone().replace("Cargo.toml", "");
    cmd.arg("count")
        .arg(&path)
        .arg("--exclude")
        .arg(format!("{}.git,{}*/test.rs,{}target", &path, &path, &path))
        .arg("-l")
        .arg("rs");
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
    let path = "/tmp/some_src_tmp.rs";
    let mut content = fs::read_to_string(&e.src_path).unwrap();
    content = content.split("\n").filter(|x| !x.starts_with("#")).collect::<Vec<&str>>().join("\n");
    fs::write(path, format_source(content.as_str())).unwrap();
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

pub fn get_caller_callee_size(e: &Extraction) -> (i32, i32) {
    let (found, mut caller, mut callee) = find_caller(e.src_path.as_str(), e.caller.as_str(), CALLEE_NAME, false);
    either!(found, panic!("did not find caller/callee"));
    let path_caller = "/tmp/some_caller_tmp.rs";
    let path_callee = "/tmp/some_callee_tmp.rs";
    caller = caller.split("\n").filter(|x| !x.trim().starts_with("#")).collect::<Vec<&str>>().join("\n");
    callee = callee.split("\n").filter(|x| !x.trim().starts_with("#")).collect::<Vec<&str>>().join("\n");
    fs::write(path_caller, caller).unwrap();
    fs::write(path_callee, callee).unwrap();
    let mut cmd_caller = Command::new("cargo");
    cmd_caller.arg("count").arg(path_caller);
    let out_caller = cmd_caller.output().unwrap();
    let mut cmd_callee = Command::new("cargo");
    cmd_callee.arg("count").arg(path_callee);
    let out_callee = cmd_callee.output().unwrap();
    if out_caller.status.success() && out_callee.status.success() {
        let stats_caller = String::from_utf8_lossy(&out_caller.stdout);
        let stats_callee = String::from_utf8_lossy(&out_callee.stdout);
        debug!("found stats: {}, {}", stats_caller.as_ref(), stats_callee.as_ref());
        let caller_size_after_ext = read_cargo_count(stats_caller.as_ref());
        let callee_size_after_ext = read_cargo_count(stats_callee.as_ref());
        (caller_size_after_ext + callee_size_after_ext, callee_size_after_ext)
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
        let res = inner_make_controls(
            extraction.src_path.as_str(),
            extraction.src_path.as_str(),
            CALLEE_NAME,
            extraction.caller.as_str(),
        );
        extraction_result.num_inputs = res.num_inputs;

        if res.has_break || res.has_continue {
            extraction_result.features_inner.push(NonLocalLoop);
        }
        if res.has_return {
            extraction_result.features_inner.push(NonLocalReturn);
        }
        res.success
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
        let res = inner_make_borrows(
            extraction.src_path.as_str(),
            extraction.src_path.as_str(),
            extraction.mut_methods_path.as_str(),
            CALLEE_NAME,
            extraction.caller.as_str(),
            extraction.original_path.as_str(),
        );

        let make_ref: Vec<String> = res
            .make_ref
            .into_iter()
            .filter(|x| !res.make_mut.contains(x))
            .collect();
        if make_ref.len() > 0 {
            extraction_result.features_inner.push(ImmutableBorrow);
        }

        if res.make_mut.len() > 0 {
            extraction_result.features_inner.push(MutableBorrow);
        }
        res.success
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
        let res = repairer.repair_project(
            extraction.src_path.as_str(),
            extraction.cargo_path.as_str(),
            CALLEE_NAME,
        );
        debug!("cargo repair counted: {}", res.repair_count);
        extraction_result.cargo_cycles = res.repair_count;
        if res.has_non_elidible_lifetime || res.repair_count > 0 {
            extraction_result.features_inner.push(NonElidibleLifetimes);
        }

        if res.has_struct_lt {
            extraction_result.features_inner.push(StructHasLifetimeSlot);
        }
        res.success
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

    let mut check = || {
        check_project(&extraction.cargo_path, &vec![])
            .output()
            .unwrap()
            .status
            .success()
    };
    time_exec("first_check", &mut check);

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
    extraction_result.features = serde_json::to_string(&extraction_result.features_inner).unwrap();

    if success {
        assert!(time_exec("final_check", &mut check).0); // in case elision or other optimization failed
    }

    (success, duration)
}
