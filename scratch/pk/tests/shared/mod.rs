use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SharedData {
    pub foo: String,
}

impl SharedData {
    pub fn new(foo: impl Into<String>) -> Self {
        Self { foo: foo.into() }
    }
}
