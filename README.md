# Sleded

A sled-ed database.

## Description

a simple crate that exposes a minimalist API for storing persistent data to a file.
Backed by sled and serde with RON.

## Example

```rs
// Something to store, needs to be serializable.
#[derive(Debug, Serialize, Deserialize)]
pub struct Student {
    name: String,
    value: usize,
}

impl TableLayout for Student {
    // adding the name of the stored table.
    fn table_name() -> String {
        "student".into()
    }
}

// storing an item
let db = open("./db").unwrap();
let table = db.table();
let bob_key = table.push(Student {
    name: "bob".into(),
    value: 0,
});

// query one item
let bob = table.get(&bob_key);
dbg!(bob);

// query all items
for (key, value) in table.iter() {
    let key = key.value(&table);
    println!("key: {key}, student: {value:?}");
}

// update items
for key in table.keys() {
    table.update(&key, |student| {
        if let Some(student) = student {
            student.value += 1;
        }
    })
}
```

## TODOs

- [x] store stuff.
- [x] table by type.
- [ ] better error handling.
- [ ] generate migrations.
