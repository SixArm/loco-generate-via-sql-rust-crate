//! Convert SQL CREATE TABLE statements to `cargo loco generate scaffold` commands.

pub(crate) mod emit;
pub(crate) mod ir;
pub(crate) mod parse;
pub(crate) mod types;

use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Postgres,
    MySql,
    SQLite,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaffoldKind {
    Htmx,
    Html,
    Api,
    None,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub dialect: Dialect,
    pub kind: ScaffoldKind,
}

impl Default for Options {
    fn default() -> Self {
        Options { dialect: Dialect::Postgres, kind: ScaffoldKind::Htmx }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error("SQL parse error: {0}")]
    Parse(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn convert(_sql: &str, _opts: &Options) -> Result<(String, Vec<Warning>), ConvertError> {
    todo!("implemented in later tasks")
}

pub fn convert_to_writer<W: Write>(
    sql: &str,
    opts: &Options,
    out: &mut W,
) -> Result<Vec<Warning>, ConvertError> {
    let (text, warnings) = convert(sql, opts)?;
    out.write_all(text.as_bytes())?;
    Ok(warnings)
}
