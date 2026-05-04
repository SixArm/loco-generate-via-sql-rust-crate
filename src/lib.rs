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

pub fn convert(sql: &str, opts: &Options) -> Result<(String, Vec<Warning>), ConvertError> {
    let tables = parse::parse_sql(sql, opts.dialect)?;
    let mut warnings: Vec<Warning> = Vec::new();
    let mut out = String::new();
    for (i, t) in tables.iter().enumerate() {
        if i > 0 {
            out.push_str("\n\n");
        }
        let cmd = emit::emit_command(t, opts, &mut warnings);
        out.push_str(&cmd);
    }
    if !out.is_empty() {
        out.push('\n');
    }
    Ok((out, warnings))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_emits_one_command_per_table_blank_separated() {
        let sql = "CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL); \
                   CREATE TABLE tags (id SERIAL PRIMARY KEY, name TEXT NOT NULL);";
        let opts = Options::default();
        let (out, warnings) = convert(sql, &opts).expect("convert");
        let expected = "\
cargo loco generate scaffold posts title:text! --htmx

cargo loco generate scaffold tags name:text! --htmx
";
        assert_eq!(out, expected);
        assert!(warnings.is_empty());
    }

    #[test]
    fn convert_returns_warnings_for_unknown_types() {
        let sql = "CREATE TABLE x (q WIDGET);";
        let (_out, warnings) = convert(sql, &Options::default()).expect("convert");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("WIDGET"));
    }

    #[test]
    fn convert_propagates_parse_error() {
        let result = convert("CREATE TABLE (oops", &Options::default());
        assert!(matches!(result, Err(ConvertError::Parse(_))));
    }

    #[test]
    fn convert_to_writer_writes_same_content() {
        let sql = "CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);";
        let opts = Options::default();
        let (expected, _) = convert(sql, &opts).expect("convert");
        let mut buf: Vec<u8> = Vec::new();
        let warnings = convert_to_writer(sql, &opts, &mut buf).expect("write");
        assert_eq!(String::from_utf8(buf).unwrap(), expected);
        assert!(warnings.is_empty());
    }

    #[test]
    fn convert_empty_input_yields_empty_output() {
        let (out, w) = convert("", &Options::default()).expect("convert");
        assert!(out.is_empty());
        assert!(w.is_empty());
    }
}
