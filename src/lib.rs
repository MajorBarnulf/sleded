use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::Db;

#[derive(Debug)]
pub enum Error {
    SledErr(sled::Error),
}

pub trait IntoTable: Serialize + DeserializeOwned {
    fn table_name<'f>() -> &'f str;
}

pub struct Table<'b, T>
where
    T: IntoTable,
{
    name: String,
    base: &'b Base,
    _item_kind: PhantomData<T>,
}

impl<'b, T> Table<'b, T>
where
    T: IntoTable,
{
    pub fn get_all(&self) -> Vec<T> {
        todo!()
    }
}

pub struct Base(Db);

impl Base {
    pub fn table<T>(&self) -> Table<T>
    where
        T: IntoTable,
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

pub fn open<P>(path: P) -> Result<Base, Error>
where
    P: AsRef<std::path::Path>,
{
    let db = sled::open(path).map_err(|e| Error::SledErr(e))?;
    Ok(Base(db))
}

#[test]
fn example() {
    #[derive(Serialize, Deserialize)]
    pub struct Student {}

    impl IntoTable for Student {
        fn table_name<'f>() -> &'f str {
            "student"
        }
    }

    let mut base = open("./db").unwrap();
    let student_table = base.table::<Student>();
    let students = student_table.get_all();
}
