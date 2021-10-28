use crate::types::{ColumnDescription, TableDescription};
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tera;

static TEMPLATE: &'static str = include_str!("./entity.py.tera");

#[derive(Serialize)]
struct TypeOrmColumn<'a> {
    desc: &'a ColumnDescription,
    ts_type: String,
}

fn exit_error(msg: String) {
    println!("{}", msg);
    std::process::exit(1);
}

impl<'a> From<&'a ColumnDescription> for TypeOrmColumn<'a> {
    fn from(c: &'a ColumnDescription) -> TypeOrmColumn<'a> {
        let t = get_scalar_py_type(&c.pg_type);
        if t.is_none() {
            exit_error(format!("unhandled type {}", c.pg_type));
        }
        let mut ts_type = t.unwrap();

        if c.is_array {
            ts_type = format!("sa.ARRAY({})", ts_type);
        }
        return TypeOrmColumn {
            desc: c,
            ts_type: ts_type,
        };
    }
}

pub fn generate_entities(tables: &Vec<TableDescription>, dest_path: &String) {
    let mut file = File::create(dest_path).unwrap();

    write_header(&mut file);

     for _ in tables
        .iter()
        .map(|t| generate_entity(&mut file, t))
    {}
}

fn write_header(file: &mut File) {
    file.write(r#"
import sqlalchemy as sa
import sqlalchemy.dialects.postgresql as pg
from sqlalchemy.ext.declarative import declarative_base
import ast
import geoalchemy2

Base = declarative_base()
metadata = Base.metadata

def _make_geometry_type(name):
    class MyType(sa.types.UserDefinedType):
        comparator_factory = geoalchemy2.comparator.Comparator

        def get_col_spec(self, **kw):
            return name

        def bind_processor(self, dialect):
            def process(value):
                return value

            return process

        def result_processor(self, dialect, coltype):
            def process(value):
                return ast.literal_eval(value)

            return process

    return MyType


_PgPolygon = _make_geometry_type("polygon")
_PgBox = _make_geometry_type("box")

"#.as_bytes()).expect("unable to write file header");
}

fn generate_entity(file: &mut File, table: &TableDescription) {
    println!("{}", table.name);
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
        .filter(|c| !c.is_pk)
        .map(|c| TypeOrmColumn::from(c))
        .collect();
    context.insert("table_name", &table.name);
    context.insert("table_name_camel_cased", &to_camel_case(&table.name));
    context.insert("schema", &table.schema);
    context.insert("columns", &cols);

    let pks: Vec<TypeOrmColumn> = table
        .columns
        .iter()
        .filter(|c| c.is_pk)
        .map(|c| TypeOrmColumn::from(c))
        .collect();

    context.insert("pk_columns", &pks);


    match tera.render_str(&TEMPLATE, &context) {
        Ok(t) => file.write_all(t.as_bytes()).unwrap(),
        Err(e) => println!("{}", e),
    };

}

fn generate_single_entity_file(table: &TableDescription, dest_path: &String) {
    let file_name = format!("{}.py", table.name);
    let file_path = Path::new(dest_path).join(&file_name);
    let mut file = File::create(&file_path).unwrap();
    write_header(&mut file);
    generate_entity(&mut file, table);
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

fn get_scalar_py_type(pg_type: &String) -> Option<String> {
    return match pg_type.replace("[]", "").as_str() {
        "boolean" => Some(String::from("sa.Boolean")),

        "integer" => Some(String::from("sa.Integer")),
        "double precision" => Some(String::from("sa.Float")),
        "single precision" => Some(String::from("sa.Float")),
        "real" => Some(String::from("sa.Float")),
        "bigint" => Some(String::from("sa.BigInteger")),

        s if s.starts_with(&"character varying") => Some(String::from("sa.String")),
        "text" => Some(String::from("sa.Text")),
        "uuid" => Some(String::from("pg.UUID")),
        s if s.starts_with(&"timestamp") && s.ends_with(&"range") => Some(String::from("pg.TSTZRANGE")),
        s if s.starts_with(&"timestamp") => Some(String::from("sa.DateTime(True)")),
        "jsonb" => Some(String::from("pg.JSONB")),
        "json" => Some(String::from("sa.JSON")),
        "interval" => Some(String::from("pg.INTERVAL")),
        "date" => Some(String::from("sa.Date")),
        "tstzrange" => Some(String::from("pg.TSTZRANGE")),
        "bytea" => Some(String::from("sa.LargeBinary")),
        "inet" => Some(String::from("pg.INET")),
        "int4range" => Some(String::from("pg.INT4RANGE")),
        "numeric" => Some(String::from("sa.Numeric")),
        "character" => Some(String::from("sa.CHAR")),
        "box" =>  Some(String::from("_PgBox")),
        "polygon" =>  Some(String::from("_PgPolygon")),
        _ => None,
    };
}
