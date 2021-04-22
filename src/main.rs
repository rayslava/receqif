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

    #[structopt(short, long, default_value = "New")]
    memo: String,

    /// Run telegram bot
    #[structopt(short, long)]
    telegram: bool,

    /// The path to the file to read
    #[structopt(required_unless = "telegram")]
    filename: Option<String>,
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

    // If program is used as command-line tool
    let acc = Account::new()
        .name("Wallet")
        .account_type(AccountType::Cash)
        .build();

    if let Some(filename) = &args.filename {
        let t = convert::convert(filename, &args.memo, &mut user, &acc).unwrap();
        print!("{}", acc.to_string());
        println!("{}", t.to_string());
    }
    #[cfg(feature = "tv")]
    {
        ui::run_tv();
    }
}
