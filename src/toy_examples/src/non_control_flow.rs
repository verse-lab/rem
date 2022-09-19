#[allow(dead_code)]

pub fn original_foo() -> String {
    let x = '1';
    let y = if x > '2' {
        x
    } else {
        return String::from("y")
    };
    String::from(y)
}

/*
pub fn new_foo() -> String {
    let x = '1';
    let y = bar_extracted(x);
    String::from(y)
}
*/

/*
fn bar_extracted(x: char) -> char {
    if x > '2' {
        x
    } else {
        return String::from("y")
    }
}
*/

pub fn new_foo_fixed() -> String {
    let x = '1';
    let y = match bar_extracted_fixed(x) {
        Ok(x) => x,
        Err(s) => return s,
    };
    String::from(y)
}

fn bar_extracted_fixed(x: char) -> Result<char, String> {
    if x > '2' {
        Ok(x)
    } else {
        Err(String::from("y"))
    }
}