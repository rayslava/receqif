#[cfg(feature = "tv")]
use std::ffi::CString;
use std::io::{stdin, stdout, Write};
#[cfg(feature = "tv")]
use std::os::raw::c_char;

#[cfg(feature = "tv")]
extern "C" {
    fn ui_main(line: *const c_char);
}

#[cfg(feature = "tv")]
#[cfg_attr(tarpaulin, ignore)]
pub fn run_tv() {
    let line = CString::new("I'm calling TV!").expect("Failed to create string");
    unsafe {
        ui_main(line.as_ptr());
    }
    println!("Hello, world!");
}

pub fn input_category(item: &str, cat: &str, cats: &[&String]) -> String {
    let mut x = String::with_capacity(64);
    if !cat.is_empty() {
        print!("'{}'? (default: {}) > ", item, cat);
    } else {
        print!(
            "'{}'? (no default, possible categories: {:?}) > ",
            item, cats
        );
    }
    let _ = stdout().flush();
    stdin().read_line(&mut x).expect("Error reading input");
    String::from(x.trim_end_matches('\n'))
}
