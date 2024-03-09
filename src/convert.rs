#[cfg(feature = "telegram")]
use crate::categories::get_top_category;
use crate::categories::CatStats;
use crate::receipt;
use crate::user::User;
use chrono::{DateTime, Utc};
use qif_generator::{account::Account, split::Split, transaction::Transaction};
use std::collections::{HashMap, HashSet};
use std::fs;

/// Read json file with receipt and convert it into `receipt::Purchase`
pub fn read_file(f: &str) -> receipt::Purchase {
    let json = fs::read_to_string(f).unwrap_or_else(|_| panic!("Can't read file {}", f));
    receipt::parse_purchase(&json)
}

/// Generate set of QIF Splits from a Purchase items
pub fn gen_splits<F, C>(
    items: &[receipt::Item],
    cs: &mut CatStats,
    accounts: &HashSet<String>,
    filter: F,
    categorizer: C,
) -> Vec<Split>
where
    C: Fn(&str, &mut CatStats, &HashSet<String>) -> String,
    F: Fn(&str) -> &str,
{
    let mut result: Vec<Split> = Vec::new();
    for i in items.iter() {
        let t = Split::new()
            .memo(filter(i.name.as_str()))
            .amount(-i.sum)
            .category(&categorizer(i.name.as_str(), cs, accounts))
            .build();

        result.push(t);

        #[cfg(feature = "monitoring")]
        crate::monitoring::PROCESSED_ITEMS.inc();
    }
    result
}

/// Generate QIF transaction from `splits`
pub fn gen_trans<'a>(
    acc: &'a Account,
    date: DateTime<Utc>,
    sum: i64,
    memo: &str,
    splits: &[Split],
) -> Result<Transaction<'a>, String> {
    let t = Transaction::new(acc)
        .date(date)
        .memo(memo)
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

/// Build a fully automatically categorized list
#[cfg(feature = "telegram")]
pub fn auto_cat_items(filename: &str, user: &User) -> (HashMap<String, String>, Vec<String>) {
    let file = read_file(filename);
    let mut categorized: HashMap<String, String> = HashMap::new();
    let mut uncategorized: Vec<String> = Vec::new();
    for i in file.items {
        if let Some(category) = get_top_category(i.name.as_str(), &user.catmap) {
            categorized.insert(i.name, category.to_string());
        } else {
            uncategorized.push(i.name)
        }
    }
    (categorized, uncategorized)
}

/// Convert `filename` into a QIF transaction
pub fn convert<'a, F, C>(
    filename: &'a str,
    memo: &str,
    user: &'a mut User,
    acc: &'a Account,
    filter: F,
    categorizer: C,
) -> Result<Transaction<'a>, String>
where
    F: Fn(&str) -> &str,
    C: Fn(&str, &mut CatStats, &HashSet<String>) -> String,
{
    let purchase = &read_file(filename);
    let splits = &gen_splits(
        &purchase.items,
        &mut user.catmap,
        &user.accounts,
        &filter,
        &categorizer,
    );
    gen_trans(acc, purchase.date(), purchase.total_sum(), memo, splits)
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

    #[test]
    fn test_read_file() {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests/resources/test.json");
        let full_path = p.to_string_lossy();

        let result = read_file(&full_path);
        assert!(!result.items.is_empty());
    }
}
