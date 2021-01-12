use chrono::{Date, Utc};
use qif_generator::{
    account::{Account, AccountType},
    split::Split,
    transaction::Transaction,
};
use std::env;
use std::fs;

mod receipt;

fn read_receipt(f: &str) -> receipt::Receipt {
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

        let result = read_receipt(&full_path).items;
        assert_eq!(result[0].name, "ХРЕН РУССКИЙ 170Г");
        assert_eq!(result[0].sum, 5549);
    }
}

fn gen_splits(items: &[receipt::Item]) -> Vec<Split> {
    let mut result: Vec<Split> = Vec::new();
    for i in items.iter() {
        let t = Split::new().memo(i.name.as_str()).amount(i.sum).build();

        result.push(t);
    }
    result
}

fn gen_trans<'a>(
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
            if t.sum() == sum {
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

#[cfg(not(tarpaulin_include))]
fn main() {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let receipt = read_receipt(filename);
    let splits = gen_splits(&receipt.items);
    let acc = Account::new()
        .name("Wallet")
        .account_type(AccountType::Cash)
        .build();

    let t = gen_trans(&acc, receipt.date(), receipt.total_sum(), &splits).unwrap();
    print!("{}", acc.to_string());
    println!("{}", t.to_string());
}
