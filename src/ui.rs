use std::io::{stdin, stdout, Write};

pub fn input_category(item: &str) -> String {
    let mut x = String::with_capacity(64);
    print!("Category for '{}'? > ", item);
    let _ = stdout().flush();
    stdin().read_line(&mut x).expect("Error reading input");
    String::from(x.trim_end_matches('\n'))
}
