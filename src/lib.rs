//! Convert SQL `CREATE TABLE` statements into `cargo loco generate scaffold` commands.
//!
//! This crate is the library half of the
//! [`loco-generate-scaffold-via-sql-schema`](https://crates.io/crates/loco-generate-scaffold-via-sql-schema)
//! tool. The binary reads SQL from stdin and writes scaffold commands to stdout;
//! the library exposes the same conversion as a function callable from any Rust
//! program.
//!
//! # What it does
//!
//! Given SQL like:
//!
//! ```sql
//! CREATE TABLE posts (
//!     id SERIAL PRIMARY KEY,
//!     title TEXT NOT NULL,
//!     slug  TEXT UNIQUE
//! );
//! ```
//!
//! it produces:
//!
//! ```text
//! cargo loco generate scaffold posts title:text! slug:text^ --htmx
//! ```
//!
//! The `id`/`created_at`/`updated_at` columns are skipped because Loco generates
//! them automatically. `NOT NULL` becomes the `!` suffix and `UNIQUE` becomes
//! the `^` suffix. `REFERENCES tbl(col)` becomes a `:references` field with one
//! of three forms (see [Foreign keys](#foreign-keys) below).
//!
//! # Quick example
//!
//! ```
//! use loco_generate_scaffold_via_sql_schema::{convert, Options};
//!
//! let sql = "CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);";
//! let (commands, warnings) = convert(sql, &Options::default()).unwrap();
//!
//! assert_eq!(
//!     commands,
//!     "cargo loco generate scaffold posts title:text! --htmx\n"
//! );
//! assert!(warnings.is_empty());
//! ```
//!
//! # Options
//!
//! [`Options`] selects the SQL [`Dialect`] (Postgres / MySQL / SQLite / Generic)
//! and the [`ScaffoldKind`] flag (Htmx / Html / Api / None) appended to each
//! command. The default is Postgres + Htmx.
//!
//! ```
//! use loco_generate_scaffold_via_sql_schema::{convert, Dialect, Options, ScaffoldKind};
//!
//! let opts = Options { dialect: Dialect::MySql, kind: ScaffoldKind::Api };
//! let sql = "CREATE TABLE widgets (id INT UNSIGNED NOT NULL, qty INT UNSIGNED NOT NULL);";
//! let (commands, _) = convert(sql, &opts).unwrap();
//!
//! assert_eq!(
//!     commands,
//!     "cargo loco generate scaffold widgets qty:unsigned! --api\n"
//! );
//! ```
//!
//! # Foreign keys
//!
//! Three rules decide how `REFERENCES tbl(col)` is rendered:
//!
//! 1. **Match.** Column ends in `_id` and the prefix matches the depluralized
//!    target table → `<prefix>:references`.
//!    `author_id REFERENCES authors(id)` → `author:references`.
//! 2. **Mismatch with `_id`.** Column ends in `_id` but the prefix doesn't
//!    match → `<singular>:references:<col>`.
//!    `owner_id REFERENCES users(id)` → `user:references:owner_id`.
//! 3. **No `_id` suffix.** → `<singular>:references:<col>`.
//!    `author REFERENCES users(id)` → `user:references:author`.
//!
//! FK columns never carry `!`/`^` suffixes — Loco's `references` template owns
//! nullability and uniqueness for those columns.
//!
//! # Warnings
//!
//! Some inputs produce warnings that are returned alongside the output rather
//! than printed. The library never writes to stderr or panics. Callers decide
//! what to do with [`Warning`] values.
//!
//! Warnings fire for:
//!
//! - Unknown SQL types (mapped to `string` and warned).
//! - `GENERATED ALWAYS AS …` columns (skipped and warned).
//!
//! # Errors
//!
//! [`convert`] returns [`ConvertError::Parse`] when sqlparser rejects the
//! input, and [`ConvertError::Io`] when [`convert_to_writer`] fails to write.
//! It never panics.
//!
//! # Source order
//!
//! Tables are emitted in the order they appear in the input. The tool does
//! not topologically sort. Arrange your input so foreign-key targets come
//! before referencing tables.
//!
//! # See also
//!
//! - The CLI binary `loco-generate-scaffold-via-sql-schema` — same conversion,
//!   driven by stdin/stdout. Run `--help` for flags.
//! - [Loco docs](https://loco.rs/docs/the-app/models/) for the canonical
//!   `cargo loco generate scaffold` syntax this tool emits.

#![forbid(unsafe_code)]

pub(crate) mod emit;
pub(crate) mod ir;
pub(crate) mod parse;
pub(crate) mod types;

use std::io::Write;

/// SQL dialect used to parse the input.
///
/// Picks which `sqlparser::dialect::*` is used. Most CREATE TABLE syntax is
/// portable across dialects; choose the dialect that matches your source SQL
/// when in doubt. `Generic` accepts the broadest grammar and is a reasonable
/// fallback for hand-written or mixed-dialect schemas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// PostgreSQL — recognizes `SERIAL`, `BIGSERIAL`, `UUID`, `JSONB`,
    /// `BYTEA`, `TIMESTAMPTZ`, schema-qualified names, etc.
    Postgres,
    /// MySQL — recognizes `UNSIGNED`, `TINYINT(1)`, `MEDIUMINT`,
    /// `MEDIUMBLOB`/`LONGBLOB`, etc.
    MySql,
    /// SQLite — recognizes `AUTOINCREMENT`, integer affinity, etc.
    SQLite,
    /// Generic — broad fallback grammar.
    Generic,
}

/// Scaffold template flag appended to each generated command.
///
/// Mirrors Loco's `--htmx` / `--html` / `--api` flags. Use [`ScaffoldKind::None`]
/// to omit the flag entirely (e.g. when piping the output through another tool
/// that adds its own flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaffoldKind {
    /// Append `--htmx`.
    Htmx,
    /// Append `--html`.
    Html,
    /// Append `--api`.
    Api,
    /// Append nothing.
    None,
}

/// Conversion options.
///
/// Use [`Options::default`] for the common case (Postgres + Htmx). Construct
/// directly when you need a different dialect or scaffold kind.
///
/// # Examples
///
/// ```
/// use loco_generate_scaffold_via_sql_schema::{Dialect, Options, ScaffoldKind};
///
/// let pg_htmx = Options::default(); // Postgres + Htmx
///
/// let mysql_api = Options {
///     dialect: Dialect::MySql,
///     kind: ScaffoldKind::Api,
/// };
/// # let _ = (pg_htmx, mysql_api);
/// ```
#[derive(Debug, Clone)]
pub struct Options {
    /// SQL dialect for parsing the input.
    pub dialect: Dialect,
    /// Scaffold flag appended to each emitted command.
    pub kind: ScaffoldKind,
}

impl Default for Options {
    /// Returns `Options { dialect: Postgres, kind: Htmx }`.
    fn default() -> Self {
        Options {
            dialect: Dialect::Postgres,
            kind: ScaffoldKind::Htmx,
        }
    }
}

/// A non-fatal observation about the input.
///
/// Conversion may succeed and still produce warnings — for example, when an
/// unknown SQL type is mapped to `string`, or a `GENERATED ALWAYS AS …`
/// column is skipped because Loco's scaffold has no equivalent.
///
/// Warnings are accumulated in source order. The library never writes them
/// to stderr; callers decide whether to print, log, or ignore them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// Human-readable description, including the table and column when known.
    pub message: String,
}

/// Errors that can occur during conversion.
///
/// The library never panics. Any failure surfaces as one of these variants.
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    /// `sqlparser` rejected the input as syntactically invalid SQL.
    ///
    /// The wrapped string is the parser's diagnostic — typically including
    /// the offending token and its position.
    #[error("SQL parse error: {0}")]
    Parse(String),

    /// An I/O error occurred while writing output (only from
    /// [`convert_to_writer`]).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convert SQL `CREATE TABLE` statements to `cargo loco generate scaffold`
/// commands.
///
/// Returns the command text plus any [`Warning`]s collected during emission.
/// The text contains one command per `CREATE TABLE`, separated by a blank line
/// and terminated by a single newline. Statements that aren't `CREATE TABLE`
/// (e.g. `CREATE INDEX`, `ALTER TABLE`) are silently skipped.
///
/// # Errors
///
/// Returns [`ConvertError::Parse`] if `sqlparser` can't parse the input.
///
/// # Examples
///
/// One table:
///
/// ```
/// use loco_generate_scaffold_via_sql_schema::{convert, Options};
///
/// let sql = "CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);";
/// let (out, warnings) = convert(sql, &Options::default()).unwrap();
///
/// assert_eq!(out, "cargo loco generate scaffold posts title:text! --htmx\n");
/// assert!(warnings.is_empty());
/// ```
///
/// Multiple tables — separated by a blank line:
///
/// ```
/// use loco_generate_scaffold_via_sql_schema::{convert, Options};
///
/// let sql = "CREATE TABLE a (id SERIAL PRIMARY KEY, x TEXT NOT NULL); \
///            CREATE TABLE b (id SERIAL PRIMARY KEY, y TEXT NOT NULL);";
/// let (out, _) = convert(sql, &Options::default()).unwrap();
///
/// assert_eq!(out, "\
/// cargo loco generate scaffold a x:text! --htmx
///
/// cargo loco generate scaffold b y:text! --htmx
/// ");
/// ```
///
/// Empty input → empty output, no error:
///
/// ```
/// use loco_generate_scaffold_via_sql_schema::{convert, Options};
///
/// let (out, w) = convert("", &Options::default()).unwrap();
/// assert!(out.is_empty());
/// assert!(w.is_empty());
/// ```
///
/// Unknown type → warning, falls back to `string`:
///
/// ```
/// use loco_generate_scaffold_via_sql_schema::{convert, Options};
///
/// let (_, warnings) = convert(
///     "CREATE TABLE x (q WIDGET);",
///     &Options::default(),
/// ).unwrap();
/// assert_eq!(warnings.len(), 1);
/// assert!(warnings[0].message.contains("WIDGET"));
/// ```
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

/// Convert SQL and stream the result into a writer.
///
/// Equivalent to calling [`convert`] and writing the resulting string to
/// `out` via [`Write::write_all`]. Returns the warnings; the command text
/// itself is consumed by the writer.
///
/// # Errors
///
/// Propagates [`ConvertError::Parse`] from parsing and
/// [`ConvertError::Io`] from the writer.
///
/// # Examples
///
/// Write into a `Vec<u8>` buffer:
///
/// ```
/// use loco_generate_scaffold_via_sql_schema::{convert_to_writer, Options};
///
/// let sql = "CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);";
/// let mut buf: Vec<u8> = Vec::new();
/// let warnings = convert_to_writer(sql, &Options::default(), &mut buf).unwrap();
///
/// assert_eq!(
///     std::str::from_utf8(&buf).unwrap(),
///     "cargo loco generate scaffold posts title:text! --htmx\n",
/// );
/// assert!(warnings.is_empty());
/// ```
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
