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
