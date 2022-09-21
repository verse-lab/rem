pub fn original_foo() -> String {
    let x = '1';
    let y = if x > '2' {
        x
    } else {
        return String::from("y")
    };
    String::from(y)
}

pub fn new_foo() -> String {
    let x = '1';
    let y = match bar_extracted(x) {
        Ok(value) => value,
        Err(value) => return value,
    };
    String::from(y)
}

fn bar_extracted(x: char) -> Result<char, String> {
    Ok(if x > '2' {
        x
    } else {
        return Err(String::from("y"))
    })
}