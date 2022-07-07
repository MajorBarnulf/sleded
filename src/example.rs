use super::*;

#[test]
fn example() {
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
