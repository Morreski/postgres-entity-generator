#[macro_use]
extern crate clap;

mod dialects;
mod types;

use clap::App;
use std::collections::HashMap;
use types::{ColumnDescription, TableDescription};

fn main() {
    let yaml = load_yaml!("../cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let pg_url = matches.value_of("URL").unwrap().to_string();
    let dialect_name = matches.value_of("dialect").unwrap().to_string();
    let out_path = matches.value_of("out").unwrap().to_string();
    let schema = matches.value_of("schema").unwrap().to_string();
    let dialect = Dialect::from_string(&dialect_name).unwrap();
    let tables = get_tables_description(&pg_url, &schema)
        .map_err(exit_error)
        .unwrap();
    match dialect {
        Dialect::TSTypeorm => dialects::ts_typeorm::generate_entities(&tables, &out_path),
    };
}

#[derive(Debug)]
enum Dialect {
    TSTypeorm,
}

impl Dialect {
    fn from_string(s: &String) -> Option<Dialect> {
        if *s == String::from("ts-typeorm") {
            return Some(Dialect::TSTypeorm);
        } else {
            return None;
        }
    }
}

fn exit_error(msg: String) {
    println!("{}", msg);
    std::process::exit(1);
}

fn get_tables_description(url: &String, schema: &String) -> Result<Vec<TableDescription>, String> {
    let mut client = postgres::Client::connect(url, postgres::NoTls).map_err(|e| e.to_string())?;
    let row_iter = client.query("
        SELECT
            c.table_schema,
            c.table_name,
            c.column_name,
            COALESCE (tco.constraint_type, '') = 'PRIMARY KEY' as is_pk,
            CASE
                WHEN c.data_type = 'ARRAY' THEN e.data_type
                ELSE c.data_type
            END as data_type,
            c.data_type = 'ARRAY' as is_array
        FROM information_schema.columns c
        LEFT JOIN information_schema.element_types e
            ON ((c.table_catalog, c.table_schema, c.table_name, 'TABLE', c.dtd_identifier)
                = (e.object_catalog, e.object_schema, e.object_name, e.object_type, e.collection_type_identifier))
        LEFT JOIN information_schema.key_column_usage as k
            ON ((c.column_name, c.table_name, c.table_schema) = (k.column_name, k.table_name, k.table_schema))
        LEFT JOIN information_schema.table_constraints tco
            ON ((k.constraint_name, k.table_name, k.table_schema) = (tco.constraint_name, tco.table_name, tco.table_schema))
        WHERE c.table_schema = $1 AND c.data_type != 'USER-DEFINED'
        ORDER BY c.table_name;", &[schema]).map_err(|e| e.to_string())?;
    let mut tables: HashMap<String, TableDescription> = HashMap::new();
    for row in row_iter {
        let column = ColumnDescription {
            name: row.get("column_name"),
            pg_type: row.get("data_type"),
            is_array: row.get("is_array"),
            is_pk: row.get("is_pk"),
        };
        let table_name: String = row.get("table_name");
        match tables.get_mut(&table_name) {
            Some(t) => {
                t.columns.push(column);
            }
            None => {
                tables.insert(
                    table_name,
                    TableDescription {
                        name: row.get("table_name"),
                        schema: row.get("table_schema"),
                        columns: Vec::from([column]),
                    },
                );
            }
        }
    }
    if tables.is_empty() {
        return Err(String::from("No tables in db"));
    }
    return Ok(tables.drain().map(|(_, v)| v).collect());
}
