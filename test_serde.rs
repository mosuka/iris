use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
enum FieldValue {
    Text(String),
    Integer(i64),
}

fn main() {
    let val = FieldValue::Integer(42);
    println!("{}", serde_json::to_string(&val).unwrap());
}
