use chrono::{Date, Utc};
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use qif_generator::{
    account::{Account, AccountType},
    split::Split,
    transaction::Transaction,
};
use shellexpand::tilde;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use structopt::StructOpt;

mod import;
mod receipt;

fn read_receipt(f: &str) -> receipt::Receipt {
    let json = fs::read_to_string(f).expect("Can't read file");
    receipt::parse_receipt(&json)
}

#[cfg(test)]
mod tests {
    use super::*;

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

/// Search for a pattern in a file and display the lines that contain it.
#[derive(StructOpt)]
struct Cli {
    /// The path to the file to read
    filename: String,

    #[structopt(parse(from_os_str), long, help = "Accounts csv file")]
    accounts: Option<PathBuf>,

    #[structopt(short, long, default_value = "~/.config/receqif/rc.db")]
    database: String,
}

#[cfg(not(tarpaulin_include))]
fn main() {
    let args = Cli::from_args();

    let confpath: &str = &tilde(&args.database);
    let confpath = PathBuf::from(confpath);

    let mut db = PickleDb::new(
        confpath,
        PickleDbDumpPolicy::PeriodicDump(Duration::from_secs(10)),
        SerializationMethod::Json,
    );

    if let Some(filename) = args.accounts {
        let accounts = import::read_accounts(Path::new(&filename)).unwrap();
        db.set("accounts", &accounts).unwrap();
    }

    let receipt = read_receipt(&args.filename);
    let splits = gen_splits(&receipt.items);
    let acc = Account::new()
        .name("Wallet")
        .account_type(AccountType::Cash)
        .build();

    let t = gen_trans(&acc, receipt.date(), receipt.total_sum(), &splits).unwrap();
    print!("{}", acc.to_string());
    println!("{}", t.to_string());
    db.dump().unwrap();
}
