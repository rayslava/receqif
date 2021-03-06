use qif_generator::account::{Account, AccountType};

use std::path::PathBuf;
use structopt::StructOpt;

mod categories;
mod convert;
mod import;
mod receipt;
#[cfg(feature = "telegram")]
mod telegram;
mod ui;
mod user;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(StructOpt)]
struct Cli {
    #[structopt(parse(from_os_str), long, help = "Accounts csv file")]
    accounts: Option<PathBuf>,

    #[structopt(short, long)]
    database: Option<String>,

    #[structopt(long, default_value = "New")]
    memo: String,

    /// Run telegram bot
    #[cfg(feature = "telegram")]
    #[structopt(short, long)]
    telegram: bool,

    /// Run Turbo Vision UI
    #[cfg(feature = "tv")]
    #[structopt(long)]
    ui: bool,

    /// The path to the file to read
    #[structopt(required_unless_one = &["telegram", "ui"])]
    filename: Option<String>,

    /// Account name
    #[structopt(long, default_value = "Wallet")]
    account: String,

    /// Account type
    #[structopt(long, parse(try_from_str), default_value = "Cash")]
    account_type: AccountType,
}

#[cfg(not(tarpaulin_include))]
fn main() {
    let args = Cli::from_args();

    #[cfg(feature = "telegram")]
    if args.telegram {
        telegram::bot();
        return;
    }

    let mut user = user::User::new(0, &args.database);

    match args.accounts {
        None => (),
        Some(path) => user.accounts(import::read_accounts(&path).unwrap()),
    }

    #[cfg(feature = "tv")]
    if args.ui {
        ui::run_tv();
        return;
    }

    // If program is used as command-line tool
    let acc = Account::new()
        .name(&args.account)
        .account_type(args.account_type)
        .build();

    if let Some(filename) = &args.filename {
        let cat = &|item: &str, stats: &mut categories::CatStats, acc: &[String]| -> String {
            categories::get_category(item, stats, acc)
        };
        let t = convert::convert(filename, &args.memo, &mut user, &acc, &cat).unwrap();
        print!("{}", acc.to_string());
        println!("{}", t.to_string());
    }
}
