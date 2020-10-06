use crate::types::{ColumnDescription, TableDescription};
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tera;

static TEMPLATE: &'static str = include_str!("./entity.ts.tera");

#[derive(Serialize)]
struct TypeOrmColumn<'a> {
    desc: &'a ColumnDescription,
    ts_type: String,
}

impl<'a> From<&'a ColumnDescription> for TypeOrmColumn<'a> {
    fn from(c: &'a ColumnDescription) -> TypeOrmColumn<'a> {
        let mut ts_type = get_scalar_ts_type(&c.pg_type).unwrap_or(String::from("any"));
        if c.is_array {
            ts_type = ts_type + &"[]";
        }
        return TypeOrmColumn {
            desc: c,
            ts_type: ts_type,
        };
    }
}

pub fn generate_entities(tables: &Vec<TableDescription>, dest_path: &String) {
    for _ in tables
        .iter()
        .map(|t| generate_single_entity_file(t, dest_path))
    {}
}

pub fn generate_single_entity_file(table: &TableDescription, dest_path: &String) {
    let mut tera = match tera::Tera::new("/dev/null*") {
        Ok(t) => t,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };
    let mut context = tera::Context::new();
    let cols: Vec<TypeOrmColumn> = table
        .columns
        .iter()
        .map(|c| TypeOrmColumn::from(c))
        .collect();
    context.insert("table_name", &table.name);
    context.insert("table_name_camel_cased", &to_camel_case(&table.name));
    context.insert("schema", &table.schema);
    context.insert("columns", &cols);
    let file_name = format!("{}.ts", table.name);
    let file_path = Path::new(dest_path).join(&file_name);
    let mut file = File::create(&file_path).unwrap();
    match tera.render_str(&TEMPLATE, &context) {
        Ok(t) => file.write_all(t.as_bytes()).unwrap(),
        Err(e) => println!("{}", e),
    };
    println!("{}", file_name);
}

fn to_camel_case(s: &String) -> String {
    let mut camel_cased = String::new();
    let mut prev: char = '_';
    let separators = ['_', ' ', '-'];
    for c in s.chars() {
        if separators.contains(&c) {
            prev = c;
            continue;
        }
        if separators.contains(&prev) {
            camel_cased.extend(c.to_uppercase());
        } else {
            camel_cased.push(c);
        }
        prev = c;
    }
    return camel_cased;
}

fn get_scalar_ts_type(pg_type: &String) -> Option<String> {
    return match pg_type.replace("[]", "").as_str() {
        "boolean" => Some(String::from("boolean")),

        "integer" => Some(String::from("number")),
        "double precision" => Some(String::from("number")),
        "bigint" => Some(String::from("BigInt")),

        "character varying" => Some(String::from("string")),
        "text" => Some(String::from("string")),
        "uuid" => Some(String::from("string")),

        s if s.ends_with(&"range") => Some(String::from("string")),
        s if s.starts_with(&"timestamp") => Some(String::from("Date")),
        "jsonb" => Some(String::from("object")),
        "json" => Some(String::from("object")),
        _ => None,
    };
}
