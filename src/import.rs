use csv::ReaderBuilder;
use std::error::Error;
use std::path::Path;

/// Read accounts from GnuCash csv export file. Only EXPENSE accounts make
/// sense, since they're used as transaction categories.
pub fn read_accounts(path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut rdr = ReaderBuilder::new().has_headers(true).from_path(path)?;
    let mut result = Vec::<String>::new();
    for e in rdr.records() {
        let record = e?;
        if record[0].eq("EXPENSE") {
            result.push(String::from(&record[1]));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{remove_file, File};
    use std::io::Write;
    use std::path::PathBuf;

    fn create_test_file() -> PathBuf {
        let file_path = PathBuf::from("test_accounts.csv");
        let mut file = File::create(&file_path).unwrap();

        writeln!(file, "Type,Name").unwrap();
        writeln!(file, "EXPENSE,Coffee").unwrap();
        writeln!(file, "INCOME,Salary").unwrap();
        writeln!(file, "EXPENSE,Books").unwrap();
        writeln!(file, "ASSET,Bank").unwrap();

        file_path
    }

    #[test]
    fn test_read_accounts() {
        let file_path = create_test_file();
        let accounts = read_accounts(&file_path).unwrap();
        assert_eq!(accounts, vec!["Coffee", "Books"]);
        remove_file(file_path).unwrap();
    }

    #[test]
    fn test_read_accounts_error() {
        let path = Path::new("non_existing_file.csv");
        assert!(read_accounts(path).is_err());
    }
}
