pub mod example;

#[derive(Debug)]
pub enum Error {
    SledErr(sled::Error),
    SerializationErr(),
}

pub mod base;
pub mod key;
pub mod table;

pub use base::{open, Base};
pub use key::{KeyOwned, KeyRef, TableKey};
pub use table::{Table, TableLayout};
