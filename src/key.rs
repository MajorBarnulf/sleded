use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::table::{Table, TableLayout};

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
    pub(crate) fn new(value: u64, _table: &'t Table<'b, T>) -> Self {
        Self { _table, value }
    }

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

impl<'t, T> TableKey<'t, T> for KeyOwned<T>
where
    T: TableLayout,
{
    fn value(&self, _table: &Table<T>) -> u64 {
        self.value
    }
}
