use std::env;
use std::fs;

mod receipt;

fn read_receipt(f: &str) -> Vec<receipt::Item> {
    let json = fs::read_to_string(f).expect("Can't read file");
    receipt::parse_receipt(&json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_read_receipt() {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests/resources/test.json");
        let full_path = p.to_string_lossy();

        let result = read_receipt(&full_path);
        assert_eq!(result[0].name, "ХРЕН РУССКИЙ 170Г");
        assert_eq!(result[0].sum, 5549);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let items = read_receipt(filename);
    for i in items.iter() {
        println!("{}", i.to_string());
    }
}
