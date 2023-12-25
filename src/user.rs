use crate::categories::CatStats;
use derive_more::From;
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use radix_trie::Trie;
use shellexpand::tilde;
use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// Configuration for single user
pub struct User {
    /// User id
    pub uid: i64,

    /// Categories statistics for the user
    pub catmap: CatStats,

    /// Available accounts for the user
    pub accounts: HashSet<String>,

    /// database with config
    db: PickleDb,
}

#[cfg(not(feature = "docker"))]
pub const DEFAULT_DB_PATH: &str = "~/.config/receqif/";

#[cfg(feature = "docker")]
pub const DEFAULT_DB_PATH: &str = "/etc/receqif/";

#[derive(Debug, Error, From)]
pub enum UserError {
    #[error("Database error: {0}")]
    DbError(#[source] pickledb::error::Error),
}

impl Drop for User {
    fn drop(&mut self) {
        self.save_data().unwrap();
    }
}

impl fmt::Debug for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("User")
            .field("uid", &self.uid)
            .field("db", &format_args!("<PickleDb>"))
            .finish()
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

        let accounts = match db.get::<Vec<String>>("accounts") {
            Some(a) => HashSet::from_iter(a),
            None => HashSet::new(),
        };

        User {
            uid,
            catmap,
            accounts,
            db,
        }
    }

    pub fn accounts(&mut self, acc: Vec<String>) {
        self.accounts = HashSet::from_iter(acc);
    }

    pub fn save_data(&mut self) -> Result<(), UserError> {
        log::debug!("Saving user data");
        self.db
            .set("catmap", &self.catmap)
            .map_err(UserError::DbError)?;

        self.db
            .set("accounts", &self.accounts)
            .map_err(UserError::DbError)?;

        self.db.dump().map_err(UserError::DbError)?;

        Ok(())
    }

    pub fn new_account(&mut self, acc: String) {
        self.accounts.insert(acc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::create_dir_all;

    const TEST_DB_DIR: &str = "/tmp/receqif_test/";

    fn setup(db_suffix: &str) -> Result<User, Box<dyn std::error::Error>> {
        create_dir_all(TEST_DB_DIR)?;

        let temp_db_path = format!("{}test_user_{}.db", TEST_DB_DIR, db_suffix);
        let user = User::new(123, &Some(temp_db_path));
        Ok(user)
    }

    #[test]
    fn test_user_initialization() {
        let user = setup("init").expect("Failed to initialize user");

        assert!(user.accounts.is_empty());
        assert_eq!(user.catmap, Trie::new());
    }

    #[test]
    fn test_adding_account() {
        let mut user = setup("add_acc").expect("Failed to set up user for adding account");

        user.new_account("account".to_string());
        assert!(user.accounts.contains("account"));
    }

    #[test]
    fn test_saving_data() {
        let mut user = setup("save_data").expect("Failed to set up user for saving data");

        user.new_account("account".to_string());
        user.save_data().expect("Failed to save data");

        // Recreate the user object to verify data persistence
        let reloaded_user = setup("save_data").expect("Failed to reload user for verifying data");

        assert!(
            reloaded_user.accounts.contains("account"),
            "Account not found in reloaded user"
        );
    }
}
