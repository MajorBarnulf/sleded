use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::Db;

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
    table: &'t Table<'b, T>,
}

impl<'t, 'b, T> KeyRef<'t, 'b, T>
where
    T: TableLayout,
{
    pub fn owned(self) -> KeyOwned<T> {
        let Self {
            value: index,
            table: _,
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
    fn value(&self, table: &Table<T>) -> u64 {
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
    fn value(&self, table: &Table<T>) -> u64 {
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

    /// TODO: refactor keys into a unique trait and have these 2 functions be generic over the trait.
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
            table: self,
        }
    }

    /// Sets an entry of the table to a certain value.
    pub fn set<'t>(&'t self, key: &KeyRef<'t, 'b, T>, value: T) {
        let key = key.value;
        self.insert(key, value);
    }

    pub fn keys<'t>(&'t self) -> impl Iterator<Item = KeyRef<'t, 'b, T>> {
        let prefix = self.table_item_path();
        let items = self.base.0.scan_prefix(&prefix);
        items.map(move |item| {
            let (key, _) = item.unwrap();
            let str = String::from_utf8_lossy(&key);
            let str = &str[prefix.len()..];
            let value = str.parse().unwrap();
            KeyRef { value, table: self }
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

#[test]
fn example() {
    use serde::Deserialize;
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Student {
        name: String,
        value: usize,
    }

    impl TableLayout for Student {
        fn table_name() -> String {
            "student".into()
        }
    }

    let base = open("./db").unwrap();
    let student_table = base.table::<Student>();

    let bobux_key = student_table.push(Student {
        name: "bobux".into(),
        value: 0,
    });

    let bobux = student_table.get(&bobux_key);
    dbg!(bobux);

    for key in student_table.keys() {
        let mut student = student_table.get(&key).unwrap();
        student.value += 1;
        student_table.set(&key, student);
    }

    let students_keys = student_table.keys();
    for key in students_keys {
        let student = student_table.get(&key);
        let key = key.value;
        println!("key: {key}, student: {student:?}");
    }
}
