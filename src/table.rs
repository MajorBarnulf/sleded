use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Serialize};

use crate::{
    base::Base,
    key::{KeyRef, TableKey},
};

/// must be implemented by types stroed in a database.
pub trait TableLayout: Serialize + DeserializeOwned {
    const TABLE_NAME: &'static str;
}

/// A table of values from an oppened database.
#[derive(Debug)]
pub struct Table<'b, T>
where
    T: TableLayout,
{
    name: &'static str,
    base: &'b Base,
    _item_kind: PhantomData<fn(T) -> T>,
}

impl<'b, T> Table<'b, T>
where
    T: TableLayout,
{
    pub(crate) fn new(base: &'b Base) -> Self {
        Self {
            _item_kind: PhantomData,
            base,
            name: T::TABLE_NAME,
        }
    }

    fn next_key(&self) -> u64 {
        let path = format!("/t/{}/next_key/", &self.name);
        let base = &self.base.inner();
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
        let base = &self.base.inner();
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
        let serialized = self.base.inner().get(path).unwrap()?;
        let deserialized = bincode::deserialize(&serialized).unwrap();
        Some(deserialized)
    }

    /// pushes a new value at the end of the table.
    pub fn push(&self, element: T) -> KeyRef<T> {
        let key = self.next_key();
        self.insert(key, element);
        KeyRef::new(key, self)
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
        self.base.inner().remove(path).unwrap();
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
        let items = self.base.inner().scan_prefix(&prefix);
        items.map(move |item| {
            let (key, _) = item.unwrap();
            let str = String::from_utf8_lossy(&key);
            let str = &str[prefix.len()..];
            let value = str.parse().unwrap();
            KeyRef::new(value, self)
        })
    }

    pub fn iter<'t>(&'t self) -> impl Iterator<Item = (KeyRef<'t, 'b, T>, T)> {
        self.keys().map(|k| {
            let v = self.get(&k).unwrap();
            (k, v)
        })
    }
}
