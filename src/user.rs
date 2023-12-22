use crate::categories::CatStats;
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use radix_trie::Trie;
use shellexpand::tilde;
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for single user
pub struct User {
    /// Categories statistics for the user
    pub catmap: CatStats,

    /// Available accounts for the user
    pub accounts: Vec<String>,

    /// database with config
    db: PickleDb,
}

#[cfg(not(feature = "docker"))]
pub const DEFAULT_DB_PATH: &str = "~/.config/receqif/";

#[cfg(feature = "docker")]
pub const DEFAULT_DB_PATH: &str = "/etc/receqif/";

impl Drop for User {
    fn drop(&mut self) {
        self.save_data();
    }
}

impl User {
    pub fn new(uid: i64, dbfile: &Option<String>) -> Self {
        let ten_sec = Duration::from_secs(10);
        let path: String = match dbfile {
            Some(path) => path.to_string(),
            None => DEFAULT_DB_PATH.to_owned() + &uid.to_string() + ".db",
        };
        let confpath: &str = &tilde(&path);
        let confpath = PathBuf::from(confpath);

        let dbase = PickleDb::load(
            &confpath,
            PickleDbDumpPolicy::PeriodicDump(ten_sec),
            SerializationMethod::Json,
        );

        let db = match dbase {
            Ok(db) => db,
            Err(_) => PickleDb::new(
                &confpath,
                PickleDbDumpPolicy::PeriodicDump(ten_sec),
                SerializationMethod::Json,
            ),
        };

        let catmap: CatStats = match db.get("catmap") {
            Some(v) => v,
            None => Trie::new(),
        };

        let accounts = match db.get("accounts") {
            Some(a) => a,
            None => vec![],
        };

        User {
            catmap,
            accounts,
            db,
        }
    }

    pub fn accounts(&mut self, acc: Vec<String>) {
        self.accounts = acc;
    }

    pub fn save_data(&mut self) {
        log::debug!("Saving user data");
        self.db.set("catmap", &self.catmap).unwrap();
        self.db.dump().unwrap();
    }
}
