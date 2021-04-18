use crate::categories::get_category;
use crate::categories::CatStats;
use crate::receipt;
use chrono::{Date, Utc};
use qif_generator::{account::Account, split::Split, transaction::Transaction};
use std::fs;

pub fn read_file(f: &str) -> receipt::Purchase {
    let json = fs::read_to_string(f).expect("Can't read file");
    receipt::parse_purchase(&json)
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

        let result = read_file(&full_path).items;
        assert_eq!(result[0].name, "СИДР 0.5 MAGNERS APP");
        assert_eq!(result[0].sum, 17713);
    }
}

pub fn gen_splits(items: &[receipt::Item], cs: &mut CatStats) -> Vec<Split> {
    let mut result: Vec<Split> = Vec::new();
    for i in items.iter() {
        let t = Split::new()
            .memo(i.name.as_str())
            .amount(-i.sum)
            .category(&get_category(i.name.as_str(), cs))
            .build();

        result.push(t);
    }
    result
}

pub fn gen_trans<'a>(
    acc: &'a Account,
    date: Date<Utc>,
    sum: i64,
    splits: &'a [Split],
) -> Result<Transaction<'a>, String> {
    let t = Transaction::new(acc)
        .date(date)
        .memo("New")
        .splits(splits)
        .build();

    match t {
        Ok(t) => {
            if t.sum() == -sum {
                Ok(t)
            } else {
                Err(format!(
                    "Total sum is wrong. Expected: {} Got: {}",
                    sum,
                    t.sum()
                ))
            }
        }
        Err(e) => Err(e),
    }
}
