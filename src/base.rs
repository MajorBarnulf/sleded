use sled::Db;

use crate::{
    table::{Table, TableLayout},
    Error,
};

/// Database type.
#[derive(Debug, Clone)]
pub struct Base(Db);

impl Base {
    pub fn inner(&self) -> &Db {
        &self.0
    }

    pub fn table<T>(&self) -> Table<T>
    where
        T: TableLayout,
    {
        Table::new(self)
    }
}

/// Opens a database or create a new if the path does not point to a valid one.
pub fn open<P>(path: P) -> Result<Base, Error>
where
    P: AsRef<std::path::Path>,
{
    let db = sled::open(path).map_err(Error::SledErr)?;
    Ok(Base(db))
}
