use crate::categories::get_category;
use crate::categories::CatStats;
use crate::receipt;
use crate::user::User;
use chrono::{Date, Utc};
use qif_generator::{account::Account, split::Split, transaction::Transaction};
use std::fs;

/// Read json file with receipt and convert it into `receipt::Purchase`
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

/// Generate set of QIF Splits from a Purchase items
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

/// Generate QIF transaction from `splits`
pub fn gen_trans<'a>(
    acc: &'a Account,
    date: Date<Utc>,
    sum: i64,
    memo: &str,
    splits: Vec<Split>,
) -> Result<Transaction<'a>, String> {
    let t = Transaction::new(acc)
        .date(date)
        .memo(memo)
        .splits(&splits)
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

/// Convert `filename` into a QIF transaction
pub fn convert<'a>(
    filename: &'a str,
    memo: &str,
    user: &'a mut User,
    acc: &'a Account,
) -> Result<Transaction<'a>, String> {
    let purchase = read_file(filename);
    let splits = gen_splits(&purchase.items, &mut user.catmap);
    gen_trans(&acc, purchase.date(), purchase.total_sum(), memo, splits)
}
