#[test]
fn example() {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Student {
        name: String,
        value: usize,
    }
    impl TableLayout for Student {
        const TABLE_NAME: &'static str = "student";
    }

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
}
