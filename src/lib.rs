use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::Db;

pub mod example;

#[derive(Debug)]
pub enum Error {
    SledErr(sled::Error),
}

/// must be implemented by types stroed in a database.
pub trait TableLayout: Serialize + DeserializeOwned {
    fn table_name() -> String;
}

/// A table of values from an oppened database.
#[derive(Debug)]
pub struct Table<'b, T>
where
    T: TableLayout,
{
    name: String,
    base: &'b Base,
    _item_kind: PhantomData<fn(T) -> T>,
}

pub trait TableKey<'t, T>
where
    T: TableLayout,
{
    fn value(&self, table: &'t Table<T>) -> u64;
}

/// key in a table with checks .
#[derive(Debug, Clone)]
pub struct KeyRef<'t, 'b, T>
where
    T: TableLayout,
{
    value: u64,
    /// TODO: compare tables before indexing as precondition
    _table: &'t Table<'b, T>,
}

impl<'t, 'b, T> KeyRef<'t, 'b, T>
where
    T: TableLayout,
{
    pub fn owned(self) -> KeyOwned<T> {
        let Self {
            value: index,
            _table: _,
        } = self;
        KeyOwned {
            value: index,
            _item_kind: PhantomData,
        }
    }
}

impl<'t, 'b, T> TableKey<'t, T> for KeyRef<'t, 'b, T>
where
    T: TableLayout,
{
    /// TODO: compare tables and catch mismatch
    fn value(&self, _table: &Table<T>) -> u64 {
        self.value
    }
}

/// key in a table that may be stored and used latter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KeyOwned<T> {
    value: u64,
    _item_kind: PhantomData<fn() -> T>,
}

impl<'t, 'b, T> TableKey<'t, T> for KeyOwned<T>
where
    T: TableLayout,
{
    fn value(&self, _table: &Table<T>) -> u64 {
        self.value
    }
}

impl<'b, T> Table<'b, T>
where
    T: TableLayout,
{
    fn next_key(&self) -> u64 {
        let path = format!("/t/{}/next_key/", &self.name);
        let base = &self.base.0;
        let serialized = base
            .fetch_and_update(path, |s| {
                let deserialized: u64 = s.map(|s| bincode::deserialize(s).unwrap()).unwrap_or(0);
                let serialized = bincode::serialize(&(deserialized + 1)).unwrap();
                Some(serialized)
            })
            .unwrap();
        serialized
            .map(|s| bincode::deserialize(&s).unwrap())
            .unwrap_or(0)
    }

    fn table_item_path(&self) -> String {
        format!("/t/{}/i/", &self.name)
    }

    /// Returns last value stored at this position if any.
    fn insert(&self, key: u64, item: T) -> Option<T> {
        let path = self.table_item_path() + &key.to_string();
        let serialized = bincode::serialize(&item).unwrap();
        let base = &self.base.0;
        base.insert(path, serialized)
            .unwrap()
            .map(|serialized| bincode::deserialize(&serialized).unwrap())
    }

    /// Gets an item from the table if it exists.
    pub fn get<'t, K>(&'t self, key: &K) -> Option<T>
    where
        K: TableKey<'t, T>,
    {
        let path = self.table_item_path() + &key.value(self).to_string();
        let serialized = self.base.0.get(path).unwrap()?;
        let deserialized = bincode::deserialize(&serialized).unwrap();
        Some(deserialized)
    }

    /// pushes a new value at the end of the table.
    pub fn push(&self, element: T) -> KeyRef<T> {
        let key = self.next_key();
        self.insert(key, element);
        KeyRef {
            value: key,
            _table: self,
        }
    }

    /// Sets an entry of the table to a certain value.
    pub fn set<'t, K>(&'t self, key: &K, value: T)
    where
        K: TableKey<'t, T>,
    {
        let key = key.value(self);
        self.insert(key, value);
    }

    pub fn delete<'t, K>(&'t self, key: &K)
    where
        K: TableKey<'t, T>,
    {
        let path = self.table_item_path() + &key.value(self).to_string();
        self.base.0.remove(path).unwrap();
    }

    /// updates an entry.
    /// If it did not existed but Some(_) is set by the closure, then it is stored.
    /// If it existed but None is set by the closure, then it is deleted.
    pub fn update<'t, K, F>(&'t self, key: &K, op: F)
    where
        K: TableKey<'t, T>,
        F: FnOnce(&mut Option<T>),
    {
        let mut value = self.get(key);
        op(&mut value);
        // update only if there is something to store
        if let Some(value) = value {
            self.set(key, value);
        } else {
            self.delete(key);
        }
    }

    pub fn keys<'t>(&'t self) -> impl Iterator<Item = KeyRef<'t, 'b, T>> {
        let prefix = self.table_item_path();
        let items = self.base.0.scan_prefix(&prefix);
        items.map(move |item| {
            let (key, _) = item.unwrap();
            let str = String::from_utf8_lossy(&key);
            let str = &str[prefix.len()..];
            let value = str.parse().unwrap();
            KeyRef {
                value,
                _table: self,
            }
        })
    }

    pub fn iter<'t>(&'t self) -> impl Iterator<Item = (KeyRef<'t, 'b, T>, T)> {
        self.keys().map(|k| {
            let v = self.get(&k).unwrap();
            (k, v)
        })
    }
}

/// Database type.
#[derive(Debug, Clone)]
pub struct Base(Db);

impl Base {
    pub fn table<T>(&self) -> Table<T>
    where
        T: TableLayout,
    {
        let name = T::table_name().to_string();
        let _item_kind = PhantomData;
        let base = self;
        Table {
            name,
            _item_kind,
            base,
        }
    }
}

/// Opens a database or create a new if the path does not point to a valid one.
pub fn open<P>(path: P) -> Result<Base, Error>
where
    P: AsRef<std::path::Path>,
{
    let db = sled::open(path).map_err(|e| Error::SledErr(e))?;
    Ok(Base(db))
}
