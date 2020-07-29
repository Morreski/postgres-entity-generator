use serde::Serialize;
use std::hash::{Hash, Hasher};

#[derive(Debug, Serialize)]
pub struct TableDescription {
    pub name: String,
    pub schema: String,
    pub columns: Vec<ColumnDescription>,
}

impl Hash for TableDescription {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.schema.hash(state);
    }
}

impl PartialEq for TableDescription {
    fn eq(&self, other: &Self) -> bool {
        return self.name == other.name && self.schema == other.schema;
    }
}

impl Eq for TableDescription {}

#[derive(Debug, Serialize)]
pub struct ColumnDescription {
    pub name: String,
    pub pg_type: String,
    pub is_array: bool,
}

impl std::fmt::Display for ColumnDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.pg_type)
    }
}
