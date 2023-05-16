use std::fmt::Write as fmtWrite;
use std::io::prelude::*;
use std::io::Write;

use log::debug;
use rem_utils::parser::ws;
use std::process::{Command, Stdio};

use crate::constraint;

pub fn parse_constraints<C: crate::constraint::LocalConstraint>(
    s: &str,
) -> nom::IResult<&str, Vec<C>> {
    use nom::{character::complete::char, multi::separated_list1, sequence::terminated};
    terminated(separated_list1(char(','), ws(C::parse)), char('.'))(s)
}

pub fn chr_solve<C: constraint::LocalConstraint>(constraints: &Vec<C>) -> Vec<C> {
    if constraints.is_empty() {
        return constraints.to_vec();
    }
    let chr_constraint_rules = C::CHR_RULES;

    let tmp =
        mktemp::Temp::new_file().expect("could not open a temp file - are you running on MacOS?");
    let tmp_path: String = tmp.to_str().unwrap().into();
    std::fs::write(&tmp_path, chr_constraint_rules.as_bytes())
        .expect("could not write chr rules to temp file");

    let mut query = "call((".to_string();
    for constraint in constraints.iter() {
        write!(query, "{},", constraint).unwrap();
    }
    // remove last char and add closing parenthesis
    debug!("query for chr: {})).", &query);
    query.pop();
    write!(query, ")).").unwrap();
    let process = Command::new("swipl")
        .arg("-q") // quiet
        .arg("-f")
        .arg(tmp_path) // open chr constraint rules
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed");

    process
        .stdin
        .unwrap()
        .write_all(query.as_bytes())
        .expect("failed");
    let mut output = String::new();
    process
        .stdout
        .unwrap()
        .read_to_string(&mut output)
        .expect("failed");
    // println!("output is:\n{}", output);
    drop(tmp);

    parse_constraints(&output)
        .expect("could not parse output of parser")
        .1
}
