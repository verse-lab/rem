use std::fs;
use std::process::Command;
use crate::repair_system::RepairSystem;

pub struct Repairer {}

fn compile_file(file_name: &str) -> Command {
    let mut compile = Command::new("rustc");
    compile
        .arg(file_name);
    compile
}

impl RepairSystem for Repairer {
    fn name(&self) -> &str {
        "simple repairer"
    }

    fn repair_file(&self, file_name: &str, new_file_name: &str) -> bool {
        fs::copy(file_name, &new_file_name).unwrap();

        loop {
            let out = compile_file(&new_file_name).output().unwrap();
            let stderr = String::from_utf8_lossy(&out.stderr);
            if stderr.len() == 0 {
                return true;
            }

            // println!("compile stdout: {}", String::from_utf8_lossy(&out.stdout));
            // println!("compile stderr: {}", stderr);

            let lines = stderr.split("\n");
            let mut help_lines: Vec<(usize, &str)> = Vec::new();
            let mut check_for_help = false;
            let mut lines_it = lines.enumerate();
            loop {
                let line = match lines_it.next() {
                    Some(line) => line,
                    None => break,
                };

                if check_for_help {
                    check_for_help = false;
                    let line_split = line.1.split(" | ");
                    let mut it = line_split.enumerate();
                    let line_number = match it.next() {
                        Some((_, line_number)) => match line_number.parse::<usize>() {
                            Ok(line_number) => line_number,
                            Err(_) => continue,
                        },
                        None => continue,
                    };
                    let line_text = match it.next() {
                        Some((_, line_text)) => line_text,
                        None => continue,
                    };
                    help_lines.push((line_number, line_text));
                }

                if line.1.starts_with("help: consider") {
                    lines_it.next(); // dump empty line
                    check_for_help = true;
                }
            }

            if help_lines.len() == 0 {
                return false;
            }

            let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
            // println!("{}", file_content);
            let lines = file_content.split("\n");
            let mut lines_modifiable = Vec::new();
            for (_, line) in lines.enumerate() {
                lines_modifiable.push(line);
            }

            for (line_number, line_text) in help_lines {
                lines_modifiable[line_number - 1] = line_text;
            }

            fs::write(new_file_name.to_string(), lines_modifiable.join("\n")).unwrap();
        }
    }
}