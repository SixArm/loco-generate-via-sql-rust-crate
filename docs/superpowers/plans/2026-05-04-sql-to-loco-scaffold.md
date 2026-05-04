# SQL → Loco Scaffold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust crate (library + thin CLI) that reads SQL `CREATE TABLE` statements from stdin and writes equivalent `cargo loco generate scaffold` commands to stdout.

**Architecture:** Pipeline of pure functions: `parse_sql` (sqlparser-based) → `Vec<TableDef>` (small dialect-neutral IR) → `emit_command` (per-table). Library never does I/O for diagnostics; the binary is the only place that touches stdin/stdout/stderr/process exit.

**Tech Stack:** Rust 2024 edition · `sqlparser` (SQL AST, dialect-aware) · `clap` derive (CLI) · `thiserror` (error type) · `assert_cmd` (binary smoke tests).

**Reference spec:** `docs/superpowers/specs/2026-05-04-sql-to-loco-scaffold-design.md`

---

## File Structure

| File | Responsibility |
|---|---|
| `Cargo.toml` | crate metadata, deps, `[lib]` + `[[bin]]` targets |
| `src/lib.rs` | public API: `Options`, `Dialect`, `ScaffoldKind`, `Warning`, `ConvertError`, `convert()`, `convert_to_writer()`. Re-exports nothing else. |
| `src/ir.rs` | dialect-neutral IR: `TableDef`, `ColumnDef`, `SqlTypeRepr`, `ForeignKey`. `pub(crate)` only. |
| `src/parse.rs` | `parse_sql(input: &str, dialect: Dialect) -> Result<Vec<TableDef>, ConvertError>`. Uses sqlparser. `pub(crate)`. |
| `src/types.rs` | `loco_type(repr: &SqlTypeRepr) -> LocoTypeResult`. Pure mapping. `pub(crate)`. |
| `src/emit.rs` | `emit_command(table: &TableDef, opts: &Options, warnings: &mut Vec<Warning>) -> String`. Pure. `pub(crate)`. |
| `src/main.rs` | CLI entry: parse args with clap, read stdin, call `convert`, write stdout, write warnings to stderr, exit codes. |
| `tests/snapshot.rs` | Whole-pipeline tests: SQL fixture in → exact stdout out. |

---

## Task 1: Cargo.toml + module skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/ir.rs`
- Create: `src/parse.rs`
- Create: `src/types.rs`
- Create: `src/emit.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write Cargo.toml**

Replace contents of `Cargo.toml`:

```toml
[package]
name = "loco-generate-scaffold-via-sql-schema"
version = "0.1.0"
edition = "2024"
description = "Convert SQL CREATE TABLE statements from stdin to `cargo loco generate scaffold` commands on stdout."
license = "MIT OR Apache-2.0"

[lib]
name = "loco_generate_scaffold_via_sql_schema"
path = "src/lib.rs"

[[bin]]
name = "loco-generate-scaffold-via-sql-schema"
path = "src/main.rs"

[dependencies]
sqlparser = "0.52"
clap = { version = "4", features = ["derive"] }
thiserror = "1"

[dev-dependencies]
assert_cmd = "2"
```

(If `cargo build` reports that `sqlparser` 0.52 is not the latest, bump to whatever `cargo search sqlparser` shows. The API surface used here — `Parser::parse_sql`, `Statement::CreateTable`, `ColumnDef`, `TableConstraint`, `DataType` — has been stable for many versions.)

- [ ] **Step 2: Create stub `src/lib.rs`**

```rust
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
```

- [ ] **Step 3: Create stub modules**

`src/ir.rs`:

```rust
//! Dialect-neutral intermediate representation.
```

`src/parse.rs`:

```rust
//! SQL → IR via `sqlparser`.
```

`src/types.rs`:

```rust
//! SQL type → Loco type-name mapping.
```

`src/emit.rs`:

```rust
//! IR → `cargo loco generate scaffold` command lines.
```

- [ ] **Step 4: Replace `src/main.rs`**

```rust
fn main() {
    eprintln!("not implemented yet");
    std::process::exit(1);
}
```

- [ ] **Step 5: Verify the crate builds**

Run: `cargo build`
Expected: PASS, no errors. (Warnings about unused `todo!` and unused modules are fine.)

Run: `cargo test --no-run`
Expected: PASS, no errors.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/
git commit -m "feat: scaffold crate (lib + bin) with module skeleton"
```

---

## Task 2: IR types

**Files:**
- Modify: `src/ir.rs`

- [ ] **Step 1: Write the failing test**

Append to `src/ir.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_a_simple_column() {
        let col = ColumnDef {
            name: "title".to_string(),
            sql_type: SqlTypeRepr {
                canonical: "VARCHAR".to_string(),
                args: vec![255],
                unsigned: false,
                array: false,
            },
            not_null: true,
            unique: false,
            primary_key: false,
            generated: false,
            fk: None,
        };
        assert_eq!(col.name, "title");
        assert_eq!(col.sql_type.canonical, "VARCHAR");
        assert!(col.not_null);
    }

    #[test]
    fn build_a_table() {
        let t = TableDef {
            name: "posts".to_string(),
            columns: vec![],
        };
        assert_eq!(t.name, "posts");
        assert!(t.columns.is_empty());
    }
}
```

- [ ] **Step 2: Run the test (expect compile failure)**

Run: `cargo test --lib ir::tests`
Expected: FAIL — `cannot find type ColumnDef`, etc.

- [ ] **Step 3: Implement IR types**

Replace `src/ir.rs` with (keeping the test module at the bottom):

```rust
//! Dialect-neutral intermediate representation.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColumnDef {
    pub name: String,
    pub sql_type: SqlTypeRepr,
    pub not_null: bool,
    pub unique: bool,
    pub primary_key: bool,
    pub generated: bool,
    pub fk: Option<ForeignKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqlTypeRepr {
    /// Upper-case base type, e.g. "VARCHAR", "TINYINT", "TIMESTAMPTZ".
    pub canonical: String,
    /// Optional length/precision args, in order. e.g. `VARCHAR(255)` → `[255]`.
    pub args: Vec<u64>,
    /// MySQL `UNSIGNED` modifier.
    pub unsigned: bool,
    /// True for `int[]` or `ARRAY` types.
    pub array: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForeignKey {
    pub target_table: String,
    pub target_column: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    // tests as above
}
```

(Replicate the test block exactly as written in Step 1 — same two tests.)

- [ ] **Step 4: Run the test**

Run: `cargo test --lib ir::tests`
Expected: PASS — 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/ir.rs
git commit -m "feat(ir): add TableDef, ColumnDef, SqlTypeRepr, ForeignKey"
```

---

## Task 3: Type mapping (SQL → Loco)

**Files:**
- Modify: `src/types.rs`
- Modify: `src/lib.rs` (add `Warning` constructor helper if missing — already there)

- [ ] **Step 1: Write the failing test**

Replace `src/types.rs` with:

```rust
//! SQL type → Loco type-name mapping.

use crate::ir::SqlTypeRepr;

/// Outcome of a type lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocoTypeResult {
    pub loco_type: &'static str,
    /// Some(reason) when we fell back to `string` for an unknown SQL type.
    pub fallback_warning: Option<String>,
}

/// Map a SQL type representation to a Loco scaffold type name.
pub(crate) fn loco_type(_repr: &SqlTypeRepr) -> LocoTypeResult {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(canonical: &str) -> SqlTypeRepr {
        SqlTypeRepr {
            canonical: canonical.to_string(),
            args: vec![],
            unsigned: false,
            array: false,
        }
    }
    fn r_args(canonical: &str, args: Vec<u64>) -> SqlTypeRepr {
        let mut x = r(canonical);
        x.args = args;
        x
    }
    fn r_unsigned(canonical: &str) -> SqlTypeRepr {
        let mut x = r(canonical);
        x.unsigned = true;
        x
    }
    fn r_array(canonical: &str) -> SqlTypeRepr {
        let mut x = r(canonical);
        x.array = true;
        x
    }

    fn assert_maps_to(repr: SqlTypeRepr, expected: &str) {
        let result = loco_type(&repr);
        assert_eq!(result.loco_type, expected, "for {:?}", repr);
        assert!(result.fallback_warning.is_none(), "unexpected warning for {:?}", repr);
    }

    #[test] fn uuid()              { assert_maps_to(r("UUID"), "uuid"); }
    #[test] fn bool_named()        { assert_maps_to(r("BOOL"), "bool"); }
    #[test] fn boolean_named()     { assert_maps_to(r("BOOLEAN"), "bool"); }
    #[test] fn tinyint_one_bool()  { assert_maps_to(r_args("TINYINT", vec![1]), "bool"); }
    #[test] fn tinyint_plain()     { assert_maps_to(r("TINYINT"), "small_int"); }
    #[test] fn smallint()          { assert_maps_to(r("SMALLINT"), "small_int"); }
    #[test] fn int2()              { assert_maps_to(r("INT2"), "small_int"); }
    #[test] fn mediumint()         { assert_maps_to(r("MEDIUMINT"), "small_int"); }
    #[test] fn int_named()         { assert_maps_to(r("INT"), "int"); }
    #[test] fn int4()              { assert_maps_to(r("INT4"), "int"); }
    #[test] fn integer()           { assert_maps_to(r("INTEGER"), "int"); }
    #[test] fn serial()            { assert_maps_to(r("SERIAL"), "int"); }
    #[test] fn bigint()            { assert_maps_to(r("BIGINT"), "big_int"); }
    #[test] fn int8()              { assert_maps_to(r("INT8"), "big_int"); }
    #[test] fn bigserial()         { assert_maps_to(r("BIGSERIAL"), "big_int"); }
    #[test] fn smallint_unsigned() { assert_maps_to(r_unsigned("SMALLINT"), "small_unsigned"); }
    #[test] fn int_unsigned()      { assert_maps_to(r_unsigned("INT"), "unsigned"); }
    #[test] fn bigint_unsigned()   { assert_maps_to(r_unsigned("BIGINT"), "big_unsigned"); }
    #[test] fn real()              { assert_maps_to(r("REAL"), "float"); }
    #[test] fn float4()            { assert_maps_to(r("FLOAT4"), "float"); }
    #[test] fn double()            { assert_maps_to(r("DOUBLE"), "double"); }
    #[test] fn double_precision()  { assert_maps_to(r("DOUBLE PRECISION"), "double"); }
    #[test] fn float8()            { assert_maps_to(r("FLOAT8"), "double"); }
    #[test] fn float_no_prec()     { assert_maps_to(r("FLOAT"), "double"); }
    #[test] fn numeric()           { assert_maps_to(r_args("NUMERIC", vec![10, 2]), "decimal"); }
    #[test] fn decimal_t()         { assert_maps_to(r("DECIMAL"), "decimal"); }
    #[test] fn money()             { assert_maps_to(r("MONEY"), "money"); }
    #[test] fn char_t()            { assert_maps_to(r("CHAR"), "string"); }
    #[test] fn character()         { assert_maps_to(r("CHARACTER"), "string"); }
    #[test] fn varchar()           { assert_maps_to(r_args("VARCHAR", vec![255]), "string"); }
    #[test] fn nvarchar()          { assert_maps_to(r("NVARCHAR"), "string"); }
    #[test] fn character_varying() { assert_maps_to(r("CHARACTER VARYING"), "string"); }
    #[test] fn text_t()            { assert_maps_to(r("TEXT"), "text"); }
    #[test] fn mediumtext()        { assert_maps_to(r("MEDIUMTEXT"), "text"); }
    #[test] fn longtext()          { assert_maps_to(r("LONGTEXT"), "text"); }
    #[test] fn clob()              { assert_maps_to(r("CLOB"), "text"); }
    #[test] fn date_t()            { assert_maps_to(r("DATE"), "date"); }
    #[test] fn time_falls_back()   { assert_maps_to(r("TIME"), "string"); }
    #[test] fn timestamp_t()       { assert_maps_to(r("TIMESTAMP"), "date_time"); }
    #[test] fn datetime_t()        { assert_maps_to(r("DATETIME"), "date_time"); }
    #[test] fn timestamptz()       { assert_maps_to(r("TIMESTAMPTZ"), "tstz"); }
    #[test] fn timestamp_with_tz() { assert_maps_to(r("TIMESTAMP WITH TIME ZONE"), "tstz"); }
    #[test] fn json_t()            { assert_maps_to(r("JSON"), "json"); }
    #[test] fn jsonb_t()           { assert_maps_to(r("JSONB"), "jsonb"); }
    #[test] fn array_named()       { assert_maps_to(r("ARRAY"), "array"); }
    #[test] fn array_postfix()     { assert_maps_to(r_array("INT"), "array"); }
    #[test] fn bytea()             { assert_maps_to(r("BYTEA"), "blob"); }
    #[test] fn blob_named()        { assert_maps_to(r("BLOB"), "blob"); }
    #[test] fn mediumblob()        { assert_maps_to(r("MEDIUMBLOB"), "blob"); }
    #[test] fn longblob()          { assert_maps_to(r("LONGBLOB"), "blob"); }
    #[test] fn binary_n()          { assert_maps_to(r_args("BINARY", vec![16]), "binary_len"); }
    #[test] fn varbinary_n()       { assert_maps_to(r_args("VARBINARY", vec![16]), "var_binary"); }

    #[test]
    fn unknown_falls_back_to_string_with_warning() {
        let result = loco_type(&r("WIDGET"));
        assert_eq!(result.loco_type, "string");
        assert!(result.fallback_warning.is_some());
        assert!(result.fallback_warning.unwrap().contains("WIDGET"));
    }
}
```

- [ ] **Step 2: Run the tests (expect failures)**

Run: `cargo test --lib types::tests`
Expected: FAIL — every test panics on the `todo!()`.

- [ ] **Step 3: Implement `loco_type`**

Replace the `loco_type` function in `src/types.rs`:

```rust
pub(crate) fn loco_type(repr: &SqlTypeRepr) -> LocoTypeResult {
    if repr.array {
        return LocoTypeResult { loco_type: "array", fallback_warning: None };
    }

    let canon = repr.canonical.to_uppercase();

    let base = match canon.as_str() {
        "UUID" => "uuid",

        "BOOL" | "BOOLEAN" => "bool",
        "TINYINT" if repr.args.first().copied() == Some(1) => "bool",
        "TINYINT" | "SMALLINT" | "INT2" | "MEDIUMINT" => "small_int",
        "INT" | "INT4" | "INTEGER" | "SERIAL" => "int",
        "BIGINT" | "INT8" | "BIGSERIAL" => "big_int",

        "REAL" | "FLOAT4" => "float",
        "DOUBLE" | "DOUBLE PRECISION" | "FLOAT8" | "FLOAT" => "double",
        "NUMERIC" | "DECIMAL" => "decimal",
        "MONEY" => "money",

        "CHAR" | "CHARACTER" | "VARCHAR" | "NVARCHAR" | "CHARACTER VARYING" => "string",
        "TEXT" | "MEDIUMTEXT" | "LONGTEXT" | "CLOB" => "text",

        "DATE" => "date",
        "TIME" => "string",
        "TIMESTAMP" | "DATETIME" => "date_time",
        "TIMESTAMPTZ" | "TIMESTAMP WITH TIME ZONE" => "tstz",

        "JSON" => "json",
        "JSONB" => "jsonb",
        "ARRAY" => "array",

        "BYTEA" | "BLOB" | "MEDIUMBLOB" | "LONGBLOB" => "blob",
        "BINARY" => "binary_len",
        "VARBINARY" => "var_binary",

        _ => {
            return LocoTypeResult {
                loco_type: "string",
                fallback_warning: Some(format!(
                    "unknown SQL type '{}', mapped to string", repr.canonical
                )),
            };
        }
    };

    // Apply unsigned modifier to integer types.
    let final_type = if repr.unsigned {
        match base {
            "small_int" => "small_unsigned",
            "int" => "unsigned",
            "big_int" => "big_unsigned",
            other => other,
        }
    } else {
        base
    };

    LocoTypeResult { loco_type: final_type, fallback_warning: None }
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib types::tests`
Expected: PASS — all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/types.rs
git commit -m "feat(types): map SQL types to canonical Loco type names"
```

---

## Task 4: SQL parsing

**Files:**
- Modify: `src/parse.rs`
- Modify: `src/lib.rs` (only if extra exports are needed — should not be)

- [ ] **Step 1: Write the failing tests**

Replace `src/parse.rs` with:

```rust
//! SQL → IR via `sqlparser`.

use crate::ConvertError;
use crate::Dialect;
use crate::ir::{ColumnDef, ForeignKey, SqlTypeRepr, TableDef};

/// Parse SQL text and return one TableDef per CREATE TABLE statement,
/// in source order. Non-CREATE-TABLE statements are skipped silently.
pub(crate) fn parse_sql(_sql: &str, _dialect: Dialect) -> Result<Vec<TableDef>, ConvertError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_pg(sql: &str) -> Vec<TableDef> {
        parse_sql(sql, Dialect::Postgres).expect("parse")
    }

    #[test]
    fn parses_a_simple_table() {
        let tables = parse_pg("CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);");
        assert_eq!(tables.len(), 1);
        let t = &tables[0];
        assert_eq!(t.name, "posts");
        assert_eq!(t.columns.len(), 2);
        assert_eq!(t.columns[0].name, "id");
        assert!(t.columns[0].primary_key);
        assert_eq!(t.columns[1].name, "title");
        assert_eq!(t.columns[1].sql_type.canonical, "TEXT");
        assert!(t.columns[1].not_null);
    }

    #[test]
    fn preserves_source_order_across_multiple_tables() {
        let sql = "CREATE TABLE a (id SERIAL PRIMARY KEY); \
                   CREATE TABLE b (id SERIAL PRIMARY KEY); \
                   CREATE TABLE c (id SERIAL PRIMARY KEY);";
        let tables = parse_pg(sql);
        let names: Vec<_> = tables.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn handles_if_not_exists() {
        let tables = parse_pg("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY);");
        assert_eq!(tables[0].name, "users");
    }

    #[test]
    fn skips_non_create_table_statements() {
        let sql = "CREATE INDEX idx ON posts(title); \
                   CREATE TABLE posts (id SERIAL PRIMARY KEY); \
                   ALTER TABLE posts ADD COLUMN x INT;";
        let tables = parse_pg(sql);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name, "posts");
    }

    #[test]
    fn detects_inline_unique() {
        let tables = parse_pg("CREATE TABLE u (email TEXT UNIQUE);");
        assert!(tables[0].columns[0].unique);
    }

    #[test]
    fn detects_table_level_single_column_unique() {
        let tables = parse_pg("CREATE TABLE u (email TEXT, UNIQUE (email));");
        assert!(tables[0].columns[0].unique);
    }

    #[test]
    fn ignores_table_level_multi_column_unique() {
        let tables = parse_pg("CREATE TABLE u (a TEXT, b TEXT, UNIQUE (a, b));");
        assert!(!tables[0].columns[0].unique);
        assert!(!tables[0].columns[1].unique);
    }

    #[test]
    fn detects_inline_references() {
        let sql = "CREATE TABLE comments (post_id INT REFERENCES posts(id));";
        let tables = parse_pg(sql);
        let fk = tables[0].columns[0].fk.as_ref().expect("fk");
        assert_eq!(fk.target_table, "posts");
        assert_eq!(fk.target_column, "id");
    }

    #[test]
    fn detects_table_level_foreign_key() {
        let sql = "CREATE TABLE comments (post_id INT, FOREIGN KEY (post_id) REFERENCES posts(id));";
        let tables = parse_pg(sql);
        let fk = tables[0].columns[0].fk.as_ref().expect("fk");
        assert_eq!(fk.target_table, "posts");
    }

    #[test]
    fn unwraps_quoted_identifiers() {
        let sql = r#"CREATE TABLE "User" ("Email" TEXT);"#;
        let tables = parse_pg(sql);
        assert_eq!(tables[0].name, "User");
        assert_eq!(tables[0].columns[0].name, "Email");
    }

    #[test]
    fn strips_schema_qualifier_from_table_name() {
        let tables = parse_pg("CREATE TABLE public.users (id SERIAL PRIMARY KEY);");
        assert_eq!(tables[0].name, "users");
    }

    #[test]
    fn captures_varchar_length() {
        let tables = parse_pg("CREATE TABLE p (title VARCHAR(255));");
        let t = &tables[0].columns[0].sql_type;
        assert_eq!(t.canonical, "VARCHAR");
        assert_eq!(t.args, vec![255]);
    }

    #[test]
    fn detects_generated_columns() {
        let sql = "CREATE TABLE p (a INT, b INT GENERATED ALWAYS AS (a + 1) STORED);";
        let tables = parse_pg(sql);
        assert!(tables[0].columns[1].generated);
    }

    #[test]
    fn mysql_unsigned_modifier() {
        let tables = parse_sql(
            "CREATE TABLE p (n INT UNSIGNED);",
            Dialect::MySql,
        ).expect("parse");
        assert!(tables[0].columns[0].sql_type.unsigned);
    }

    #[test]
    fn parse_error_is_reported() {
        let result = parse_sql("CREATE TABLE (bad", Dialect::Postgres);
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run the tests (expect failures)**

Run: `cargo test --lib parse::tests`
Expected: FAIL — `todo!()` panics.

- [ ] **Step 3: Implement `parse_sql`**

Replace the body of `src/parse.rs` (keep the test module unchanged):

```rust
//! SQL → IR via `sqlparser`.

use crate::ConvertError;
use crate::Dialect;
use crate::ir::{ColumnDef, ForeignKey, SqlTypeRepr, TableDef};

use sqlparser::ast::{
    ColumnOption, DataType, ExactNumberInfo, GeneratedAs, ObjectName,
    Statement, TableConstraint, TimezoneInfo,
};
use sqlparser::dialect::{
    Dialect as SqlDialect, GenericDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect,
};
use sqlparser::parser::Parser;

pub(crate) fn parse_sql(sql: &str, dialect: Dialect) -> Result<Vec<TableDef>, ConvertError> {
    let d: Box<dyn SqlDialect> = match dialect {
        Dialect::Postgres => Box::new(PostgreSqlDialect {}),
        Dialect::MySql => Box::new(MySqlDialect {}),
        Dialect::SQLite => Box::new(SQLiteDialect {}),
        Dialect::Generic => Box::new(GenericDialect {}),
    };
    let stmts = Parser::parse_sql(&*d, sql).map_err(|e| ConvertError::Parse(e.to_string()))?;

    let mut out = Vec::new();
    for stmt in stmts {
        if let Statement::CreateTable(ct) = stmt {
            out.push(table_from_create(&ct.name, &ct.columns, &ct.constraints));
        }
    }
    Ok(out)
}

fn table_from_create(
    name: &ObjectName,
    columns: &[sqlparser::ast::ColumnDef],
    constraints: &[TableConstraint],
) -> TableDef {
    // Use the last segment of a possibly-qualified name (`public.users` → `users`).
    let table_name = name
        .0
        .last()
        .map(|i| i.value.clone())
        .unwrap_or_default();

    // Pre-scan constraints for single-column UNIQUE and FOREIGN KEY.
    let mut unique_singles: Vec<String> = Vec::new();
    let mut tbl_fks: Vec<(String, ForeignKey)> = Vec::new();
    for c in constraints {
        match c {
            TableConstraint::Unique { columns, .. } if columns.len() == 1 => {
                unique_singles.push(columns[0].value.clone());
            }
            TableConstraint::ForeignKey {
                columns,
                foreign_table,
                referred_columns,
                ..
            } if columns.len() == 1 && referred_columns.len() == 1 => {
                let target_table = foreign_table
                    .0.last().map(|i| i.value.clone()).unwrap_or_default();
                let target_column = referred_columns[0].value.clone();
                tbl_fks.push((columns[0].value.clone(), ForeignKey { target_table, target_column }));
            }
            _ => {}
        }
    }

    let mut cols = Vec::with_capacity(columns.len());
    for c in columns {
        let mut col = column_from_sqlparser(c);
        if unique_singles.iter().any(|n| n == &col.name) {
            col.unique = true;
        }
        if col.fk.is_none() {
            if let Some((_, fk)) = tbl_fks.iter().find(|(n, _)| n == &col.name) {
                col.fk = Some(fk.clone());
            }
        }
        cols.push(col);
    }

    TableDef { name: table_name, columns: cols }
}

fn column_from_sqlparser(c: &sqlparser::ast::ColumnDef) -> ColumnDef {
    let mut not_null = false;
    let mut unique = false;
    let mut primary_key = false;
    let mut generated = false;
    let mut fk: Option<ForeignKey> = None;

    for opt in &c.options {
        match &opt.option {
            ColumnOption::NotNull => not_null = true,
            ColumnOption::Unique { is_primary, .. } => {
                if *is_primary {
                    primary_key = true;
                    not_null = true;
                } else {
                    unique = true;
                }
            }
            ColumnOption::ForeignKey { foreign_table, referred_columns, .. } => {
                let target_table = foreign_table.0.last().map(|i| i.value.clone()).unwrap_or_default();
                let target_column = referred_columns
                    .first().map(|i| i.value.clone()).unwrap_or_else(|| "id".to_string());
                fk = Some(ForeignKey { target_table, target_column });
            }
            ColumnOption::Generated { generated_as: GeneratedAs::Always, .. }
            | ColumnOption::Generated { generated_as: GeneratedAs::ByDefault, .. } => {
                generated = true;
            }
            _ => {}
        }
    }

    let sql_type = sql_type_repr(&c.data_type);
    ColumnDef {
        name: c.name.value.clone(),
        sql_type,
        not_null,
        unique,
        primary_key,
        generated,
        fk,
    }
}

fn sql_type_repr(dt: &DataType) -> SqlTypeRepr {
    use DataType::*;
    let (canonical, args, unsigned, array) = match dt {
        Uuid => ("UUID".to_string(), vec![], false, false),
        Boolean => ("BOOL".to_string(), vec![], false, false),
        TinyInt(n) => ("TINYINT".to_string(), n.iter().copied().map(|n| n as u64).collect(), false, false),
        UnsignedTinyInt(n) => ("TINYINT".to_string(), n.iter().copied().map(|n| n as u64).collect(), true, false),
        SmallInt(n) => ("SMALLINT".to_string(), n.iter().copied().map(|n| n as u64).collect(), false, false),
        UnsignedSmallInt(n) => ("SMALLINT".to_string(), n.iter().copied().map(|n| n as u64).collect(), true, false),
        MediumInt(n) => ("MEDIUMINT".to_string(), n.iter().copied().map(|n| n as u64).collect(), false, false),
        UnsignedMediumInt(n) => ("MEDIUMINT".to_string(), n.iter().copied().map(|n| n as u64).collect(), true, false),
        Int(n) | Integer(n) => ("INT".to_string(), n.iter().copied().map(|n| n as u64).collect(), false, false),
        UnsignedInt(n) | UnsignedInteger(n) => ("INT".to_string(), n.iter().copied().map(|n| n as u64).collect(), true, false),
        BigInt(n) => ("BIGINT".to_string(), n.iter().copied().map(|n| n as u64).collect(), false, false),
        UnsignedBigInt(n) => ("BIGINT".to_string(), n.iter().copied().map(|n| n as u64).collect(), true, false),
        Real => ("REAL".to_string(), vec![], false, false),
        Double => ("DOUBLE".to_string(), vec![], false, false),
        DoublePrecision => ("DOUBLE PRECISION".to_string(), vec![], false, false),
        Float(_) => ("FLOAT".to_string(), vec![], false, false),
        Numeric(info) => ("NUMERIC".to_string(), exact_args(info), false, false),
        Decimal(info) => ("DECIMAL".to_string(), exact_args(info), false, false),
        Char(n) | Character(n) => ("CHAR".to_string(), n.iter().filter_map(|x| x.length.map(|l| l as u64)).collect(), false, false),
        Varchar(n) | CharVarying(n) | CharacterVarying(n) => ("VARCHAR".to_string(), n.iter().filter_map(|x| x.length.map(|l| l as u64)).collect(), false, false),
        Nvarchar(n) => ("NVARCHAR".to_string(), n.iter().filter_map(|x| x.length.map(|l| l as u64)).collect(), false, false),
        Text => ("TEXT".to_string(), vec![], false, false),
        MediumText => ("MEDIUMTEXT".to_string(), vec![], false, false),
        LongText => ("LONGTEXT".to_string(), vec![], false, false),
        Clob(n) => ("CLOB".to_string(), n.iter().copied().map(|n| n as u64).collect(), false, false),
        Date => ("DATE".to_string(), vec![], false, false),
        Time(_, _) => ("TIME".to_string(), vec![], false, false),
        Datetime(_) => ("DATETIME".to_string(), vec![], false, false),
        Timestamp(_, TimezoneInfo::Tz)
        | Timestamp(_, TimezoneInfo::WithTimeZone) => ("TIMESTAMPTZ".to_string(), vec![], false, false),
        Timestamp(_, _) => ("TIMESTAMP".to_string(), vec![], false, false),
        JSON => ("JSON".to_string(), vec![], false, false),
        JSONB => ("JSONB".to_string(), vec![], false, false),
        Bytea => ("BYTEA".to_string(), vec![], false, false),
        Blob(n) => ("BLOB".to_string(), n.iter().copied().map(|n| n as u64).collect(), false, false),
        Binary(n) => ("BINARY".to_string(), n.iter().copied().map(|n| n as u64).collect(), false, false),
        Varbinary(n) => ("VARBINARY".to_string(), n.iter().copied().map(|n| n as u64).collect(), false, false),
        Array(_) => ("ARRAY".to_string(), vec![], false, true),
        // Catch-all: stringify whatever sqlparser saw.
        other => (format!("{}", other).to_uppercase(), vec![], false, false),
    };

    SqlTypeRepr { canonical, args, unsigned, array }
}

fn exact_args(info: &ExactNumberInfo) -> Vec<u64> {
    match info {
        ExactNumberInfo::None => vec![],
        ExactNumberInfo::Precision(p) => vec![*p as u64],
        ExactNumberInfo::PrecisionAndScale(p, s) => vec![*p as u64, *s as u64],
    }
}
```

(If the installed sqlparser version differs and a variant name does not match — for example `Int(_)` may be `Int(Option<u64>)` or carry a different inner type, or `Timestamp` may take different arguments — adjust the variant pattern to compile. The structure of the function does not change: each arm sets `(canonical, args, unsigned, array)`. The `other => format!(...)` arm preserves correctness for anything not enumerated above by stringifying the type, which then falls through to `string` in `types.rs` with a warning.)

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib parse::tests`
Expected: PASS — all parse tests pass.

(If a test fails because sqlparser's surface differs in the installed version, adjust the affected match arm and re-run. Do not weaken the test — fix the implementation.)

- [ ] **Step 5: Commit**

```bash
git add src/parse.rs
git commit -m "feat(parse): SQL CREATE TABLE → IR via sqlparser"
```

---

## Task 5: Field-spec emission (per-column)

**Files:**
- Modify: `src/emit.rs`

- [ ] **Step 1: Write the failing test**

Replace `src/emit.rs` with:

```rust
//! IR → `cargo loco generate scaffold` command lines.

use crate::Options;
use crate::Warning;
use crate::ir::{ColumnDef, ForeignKey, SqlTypeRepr, TableDef};
use crate::types::loco_type;

const SKIP_COLUMNS: &[&str] = &["id", "created_at", "updated_at"];

/// Render the per-column field spec, or None if the column is skipped.
pub(crate) fn emit_field(col: &ColumnDef, warnings: &mut Vec<Warning>, table_name: &str) -> Option<String> {
    let _ = (col, warnings, table_name);
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str, canonical: &str) -> ColumnDef {
        ColumnDef {
            name: name.to_string(),
            sql_type: SqlTypeRepr {
                canonical: canonical.to_string(),
                args: vec![],
                unsigned: false,
                array: false,
            },
            not_null: false,
            unique: false,
            primary_key: false,
            generated: false,
            fk: None,
        }
    }

    fn run(c: ColumnDef) -> (Option<String>, Vec<Warning>) {
        let mut w = Vec::new();
        let f = emit_field(&c, &mut w, "t");
        (f, w)
    }

    #[test]
    fn skips_id() {
        let (f, _) = run(col("id", "INT"));
        assert!(f.is_none());
    }

    #[test]
    fn skips_created_at() {
        let (f, _) = run(col("created_at", "TIMESTAMPTZ"));
        assert!(f.is_none());
    }

    #[test]
    fn skips_updated_at() {
        let (f, _) = run(col("updated_at", "TIMESTAMPTZ"));
        assert!(f.is_none());
    }

    #[test]
    fn skips_generated_columns_with_warning() {
        let mut c = col("computed", "INT");
        c.generated = true;
        let (f, w) = run(c);
        assert!(f.is_none());
        assert_eq!(w.len(), 1);
        assert!(w[0].message.contains("computed"));
    }

    #[test]
    fn plain_string_no_suffix() {
        let (f, _) = run(col("title", "TEXT"));
        assert_eq!(f.as_deref(), Some("title:text"));
    }

    #[test]
    fn not_null_adds_bang() {
        let mut c = col("title", "VARCHAR");
        c.not_null = true;
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("title:string!"));
    }

    #[test]
    fn unique_adds_caret() {
        let mut c = col("email", "TEXT");
        c.unique = true;
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("email:text^"));
    }

    #[test]
    fn not_null_unique_appends_both() {
        let mut c = col("email", "TEXT");
        c.not_null = true;
        c.unique = true;
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("email:text!^"));
    }

    #[test]
    fn fk_rule_1_matching_prefix_uses_bare_references() {
        let mut c = col("author_id", "INT");
        c.fk = Some(ForeignKey { target_table: "authors".into(), target_column: "id".into() });
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("author:references"));
    }

    #[test]
    fn fk_rule_1_matches_singular_target_table() {
        // target table happens to already be singular
        let mut c = col("author_id", "INT");
        c.fk = Some(ForeignKey { target_table: "author".into(), target_column: "id".into() });
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("author:references"));
    }

    #[test]
    fn fk_rule_2_mismatched_prefix_uses_custom_col() {
        let mut c = col("owner_id", "INT");
        c.fk = Some(ForeignKey { target_table: "users".into(), target_column: "id".into() });
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("user:references:owner_id"));
    }

    #[test]
    fn fk_rule_3_no_id_suffix_uses_custom_col() {
        let mut c = col("author", "INT");
        c.fk = Some(ForeignKey { target_table: "users".into(), target_column: "id".into() });
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("user:references:author"));
    }

    #[test]
    fn fk_strips_not_null_and_unique_suffixes() {
        let mut c = col("author_id", "INT");
        c.not_null = true;
        c.unique = true;
        c.fk = Some(ForeignKey { target_table: "authors".into(), target_column: "id".into() });
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("author:references"));
    }

    #[test]
    fn unknown_type_falls_back_to_string_with_warning() {
        let (f, w) = run(col("x", "WIDGET"));
        assert_eq!(f.as_deref(), Some("x:string"));
        assert_eq!(w.len(), 1);
        assert!(w[0].message.contains("WIDGET"));
    }
}
```

- [ ] **Step 2: Run the tests (expect failures)**

Run: `cargo test --lib emit::tests`
Expected: FAIL — `todo!()` panics.

- [ ] **Step 3: Implement `emit_field`**

Replace the body of `src/emit.rs` (keep the test module unchanged):

```rust
//! IR → `cargo loco generate scaffold` command lines.

use crate::Options;
use crate::ScaffoldKind;
use crate::Warning;
use crate::ir::{ColumnDef, ForeignKey, SqlTypeRepr, TableDef};
use crate::types::loco_type;

const SKIP_COLUMNS: &[&str] = &["id", "created_at", "updated_at"];

pub(crate) fn emit_field(col: &ColumnDef, warnings: &mut Vec<Warning>, table_name: &str) -> Option<String> {
    if SKIP_COLUMNS.contains(&col.name.as_str()) {
        return None;
    }
    if col.generated {
        warnings.push(Warning {
            message: format!(
                "skipping generated column '{}.{}' (Loco scaffold has no equivalent)",
                table_name, col.name,
            ),
        });
        return None;
    }

    if let Some(fk) = &col.fk {
        return Some(emit_fk_field(&col.name, fk));
    }

    let mapped = loco_type(&col.sql_type);
    if let Some(msg) = mapped.fallback_warning {
        warnings.push(Warning {
            message: format!("{} (column '{}.{}')", msg, table_name, col.name),
        });
    }

    let mut suffix = String::new();
    if col.not_null || col.primary_key {
        suffix.push('!');
    }
    if col.unique {
        suffix.push('^');
    }

    Some(format!("{}:{}{}", col.name, mapped.loco_type, suffix))
}

/// Implements the three FK rules from the spec.
fn emit_fk_field(col_name: &str, fk: &ForeignKey) -> String {
    let target_singular = depluralize(&fk.target_table);

    if let Some(prefix) = col_name.strip_suffix("_id") {
        let target_match = depluralize(&fk.target_table);
        if prefix == target_match {
            return format!("{}:references", prefix);
        }
        // Rule 2: ends in _id but prefix doesn't match.
        return format!("{}:references:{}", target_singular, col_name);
    }

    // Rule 3: column does not end in _id.
    format!("{}:references:{}", target_singular, col_name)
}

fn depluralize(s: &str) -> String {
    if let Some(stem) = s.strip_suffix('s') {
        stem.to_string()
    } else {
        s.to_string()
    }
}

// (test module unchanged from Step 1)
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib emit::tests`
Expected: PASS — all field tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/emit.rs
git commit -m "feat(emit): per-column field spec with skip/suffix/FK rules"
```

---

## Task 6: Whole-command emission

**Files:**
- Modify: `src/emit.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` module in `src/emit.rs`:

```rust
    fn make_table() -> TableDef {
        let mut id = col("id", "INT");
        id.primary_key = true;
        let mut title = col("title", "VARCHAR");
        title.not_null = true;
        let mut email = col("email", "TEXT");
        email.unique = true;
        TableDef {
            name: "posts".into(),
            columns: vec![id, title, email],
        }
    }

    #[test]
    fn emit_command_with_htmx_kind() {
        let t = make_table();
        let opts = Options { dialect: crate::Dialect::Postgres, kind: ScaffoldKind::Htmx };
        let mut w = Vec::new();
        let cmd = emit_command(&t, &opts, &mut w);
        assert_eq!(
            cmd,
            "cargo loco generate scaffold posts title:string! email:text^ --htmx"
        );
        assert!(w.is_empty());
    }

    #[test]
    fn emit_command_with_html_kind() {
        let t = make_table();
        let opts = Options { dialect: crate::Dialect::Postgres, kind: ScaffoldKind::Html };
        let mut w = Vec::new();
        let cmd = emit_command(&t, &opts, &mut w);
        assert!(cmd.ends_with(" --html"));
    }

    #[test]
    fn emit_command_with_api_kind() {
        let t = make_table();
        let opts = Options { dialect: crate::Dialect::Postgres, kind: ScaffoldKind::Api };
        let mut w = Vec::new();
        let cmd = emit_command(&t, &opts, &mut w);
        assert!(cmd.ends_with(" --api"));
    }

    #[test]
    fn emit_command_with_kind_none_omits_flag() {
        let t = make_table();
        let opts = Options { dialect: crate::Dialect::Postgres, kind: ScaffoldKind::None };
        let mut w = Vec::new();
        let cmd = emit_command(&t, &opts, &mut w);
        assert_eq!(
            cmd,
            "cargo loco generate scaffold posts title:string! email:text^"
        );
    }

    #[test]
    fn emit_command_table_with_no_user_columns() {
        let t = TableDef {
            name: "tags".into(),
            columns: vec![{
                let mut id = col("id", "INT");
                id.primary_key = true;
                id
            }],
        };
        let opts = Options { dialect: crate::Dialect::Postgres, kind: ScaffoldKind::Htmx };
        let mut w = Vec::new();
        let cmd = emit_command(&t, &opts, &mut w);
        assert_eq!(cmd, "cargo loco generate scaffold tags --htmx");
    }
```

- [ ] **Step 2: Run the tests (expect failure)**

Run: `cargo test --lib emit::tests`
Expected: FAIL — `cannot find function emit_command`.

- [ ] **Step 3: Implement `emit_command`**

Add to `src/emit.rs` (above the `#[cfg(test)]` line):

```rust
pub(crate) fn emit_command(table: &TableDef, opts: &Options, warnings: &mut Vec<Warning>) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push("cargo loco generate scaffold".to_string());
    parts.push(table.name.clone());

    for col in &table.columns {
        if let Some(field) = emit_field(col, warnings, &table.name) {
            parts.push(field);
        }
    }

    match opts.kind {
        ScaffoldKind::Htmx => parts.push("--htmx".to_string()),
        ScaffoldKind::Html => parts.push("--html".to_string()),
        ScaffoldKind::Api  => parts.push("--api".to_string()),
        ScaffoldKind::None => {}
    }

    parts.join(" ")
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib emit::tests`
Expected: PASS — all emit tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/emit.rs
git commit -m "feat(emit): assemble whole `cargo loco generate scaffold` command"
```

---

## Task 7: Public `convert` API

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to `src/lib.rs`:

```rust
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
```

- [ ] **Step 2: Run the tests (expect failure)**

Run: `cargo test --lib tests::`
Expected: FAIL — `convert` panics on `todo!()`.

- [ ] **Step 3: Implement `convert`**

Replace the `convert` function in `src/lib.rs` with:

```rust
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
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib`
Expected: PASS — every lib test passes.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs
git commit -m "feat(lib): public convert() and convert_to_writer() API"
```

---

## Task 8: CLI binary

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write the binary**

Replace `src/main.rs`:

```rust
use std::io::{Read, Write};
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use loco_generate_scaffold_via_sql_schema::{
    ConvertError, Dialect, Options, ScaffoldKind, convert,
};

/// Convert SQL CREATE TABLE statements from stdin into
/// `cargo loco generate scaffold` commands on stdout.
#[derive(Debug, Parser)]
#[command(name = "loco-generate-scaffold-via-sql-schema", version, about)]
struct Args {
    /// SQL dialect to parse with.
    #[arg(short, long, value_enum, default_value_t = DialectArg::Postgres)]
    dialect: DialectArg,

    /// Scaffold template kind to append to each command.
    #[arg(short, long, value_enum, default_value_t = KindArg::Htmx)]
    kind: KindArg,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum DialectArg { Postgres, Mysql, Sqlite, Generic }

impl From<DialectArg> for Dialect {
    fn from(d: DialectArg) -> Self {
        match d {
            DialectArg::Postgres => Dialect::Postgres,
            DialectArg::Mysql    => Dialect::MySql,
            DialectArg::Sqlite   => Dialect::SQLite,
            DialectArg::Generic  => Dialect::Generic,
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum KindArg { Htmx, Html, Api, None }

impl From<KindArg> for ScaffoldKind {
    fn from(k: KindArg) -> Self {
        match k {
            KindArg::Htmx => ScaffoldKind::Htmx,
            KindArg::Html => ScaffoldKind::Html,
            KindArg::Api  => ScaffoldKind::Api,
            KindArg::None => ScaffoldKind::None,
        }
    }
}

fn main() -> ExitCode {
    let args = Args::parse();

    let mut sql = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut sql) {
        eprintln!("error: failed to read stdin: {}", e);
        return ExitCode::from(2);
    }

    let opts = Options { dialect: args.dialect.into(), kind: args.kind.into() };

    match convert(&sql, &opts) {
        Ok((out, warnings)) => {
            for w in &warnings {
                eprintln!("warn: {}", w.message);
            }
            if let Err(e) = std::io::stdout().write_all(out.as_bytes()) {
                eprintln!("error: failed to write stdout: {}", e);
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Err(ConvertError::Parse(msg)) => {
            eprintln!("error: SQL parse failed: {}", msg);
            ExitCode::from(1)
        }
        Err(ConvertError::Io(e)) => {
            eprintln!("error: {}", e);
            ExitCode::from(2)
        }
    }
}
```

- [ ] **Step 2: Verify it builds**

Run: `cargo build`
Expected: PASS, no errors.

- [ ] **Step 3: Manual smoke test**

Run:
```sh
echo 'CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);' \
  | cargo run --quiet
```
Expected stdout (exactly):
```
cargo loco generate scaffold posts title:text! --htmx
```

Run:
```sh
echo 'CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);' \
  | cargo run --quiet -- --kind api
```
Expected stdout (exactly):
```
cargo loco generate scaffold posts title:text! --api
```

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat(cli): clap-based binary reads stdin, writes stdout"
```

---

## Task 9: Integration snapshot tests

**Files:**
- Create: `tests/snapshot.rs`

- [ ] **Step 1: Write integration tests**

Create `tests/snapshot.rs`:

```rust
use loco_generate_scaffold_via_sql_schema::{Dialect, Options, ScaffoldKind, convert};

fn opts(d: Dialect, k: ScaffoldKind) -> Options {
    Options { dialect: d, kind: k }
}

#[test]
fn simple_postgres_table() {
    let sql = "\
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    body  TEXT NOT NULL,
    slug  TEXT UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
";
    let (out, w) = convert(sql, &opts(Dialect::Postgres, ScaffoldKind::Htmx)).unwrap();
    assert_eq!(
        out,
        "cargo loco generate scaffold posts title:string! body:text! slug:text^ --htmx\n"
    );
    assert!(w.is_empty());
}

#[test]
fn fk_chain_preserves_source_order() {
    let sql = "\
CREATE TABLE authors (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    author_id INT NOT NULL REFERENCES authors(id),
    title TEXT NOT NULL
);
CREATE TABLE comments (
    id SERIAL PRIMARY KEY,
    post_id INT NOT NULL REFERENCES posts(id),
    owner_id INT NOT NULL REFERENCES authors(id),
    body TEXT NOT NULL
);
";
    let (out, _) = convert(sql, &opts(Dialect::Postgres, ScaffoldKind::Htmx)).unwrap();
    let expected = "\
cargo loco generate scaffold authors name:text! --htmx

cargo loco generate scaffold posts author:references title:text! --htmx

cargo loco generate scaffold comments post:references author:references:owner_id body:text! --htmx
";
    assert_eq!(out, expected);
}

#[test]
fn mysql_unsigned_and_tinyint_one() {
    let sql = "\
CREATE TABLE widgets (
    id INT UNSIGNED NOT NULL,
    qty INT UNSIGNED NOT NULL,
    big BIGINT UNSIGNED NOT NULL,
    active TINYINT(1) NOT NULL
);
";
    let (out, _) = convert(sql, &opts(Dialect::MySql, ScaffoldKind::Htmx)).unwrap();
    assert_eq!(
        out,
        "cargo loco generate scaffold widgets qty:unsigned! big:big_unsigned! active:bool! --htmx\n"
    );
}

#[test]
fn sqlite_autoincrement() {
    let sql = "\
CREATE TABLE notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    body  TEXT
);
";
    let (out, _) = convert(sql, &opts(Dialect::SQLite, ScaffoldKind::Htmx)).unwrap();
    assert_eq!(
        out,
        "cargo loco generate scaffold notes title:text! body:text --htmx\n"
    );
}

#[test]
fn multi_column_unique_is_dropped_silently() {
    let sql = "\
CREATE TABLE memberships (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    group_id INT NOT NULL,
    UNIQUE (user_id, group_id)
);
";
    let (out, w) = convert(sql, &opts(Dialect::Postgres, ScaffoldKind::Htmx)).unwrap();
    // Neither user_id nor group_id should carry `^`.
    assert_eq!(
        out,
        "cargo loco generate scaffold memberships user_id:int! group_id:int! --htmx\n"
    );
    assert!(w.is_empty());
}

#[test]
fn generated_column_is_skipped_with_warning() {
    let sql = "\
CREATE TABLE p (
    id SERIAL PRIMARY KEY,
    a INT NOT NULL,
    b INT GENERATED ALWAYS AS (a + 1) STORED
);
";
    let (out, w) = convert(sql, &opts(Dialect::Postgres, ScaffoldKind::Htmx)).unwrap();
    assert_eq!(out, "cargo loco generate scaffold p a:int! --htmx\n");
    assert_eq!(w.len(), 1);
    assert!(w[0].message.contains("p.b"));
}

#[test]
fn schema_qualified_table_name_uses_last_segment() {
    let sql = "CREATE TABLE public.users (id SERIAL PRIMARY KEY, email TEXT NOT NULL UNIQUE);";
    let (out, _) = convert(sql, &opts(Dialect::Postgres, ScaffoldKind::Htmx)).unwrap();
    assert_eq!(out, "cargo loco generate scaffold users email:text!^ --htmx\n");
}

#[test]
fn binary_smoke_test_via_assert_cmd() {
    use assert_cmd::Command;
    let mut cmd = Command::cargo_bin("loco-generate-scaffold-via-sql-schema").unwrap();
    cmd.write_stdin("CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);")
        .assert()
        .success()
        .stdout("cargo loco generate scaffold posts title:text! --htmx\n");
}

#[test]
fn binary_smoke_test_kind_none() {
    use assert_cmd::Command;
    let mut cmd = Command::cargo_bin("loco-generate-scaffold-via-sql-schema").unwrap();
    cmd.args(["--kind", "none"])
        .write_stdin("CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);")
        .assert()
        .success()
        .stdout("cargo loco generate scaffold posts title:text!\n");
}

#[test]
fn binary_exits_nonzero_on_parse_error() {
    use assert_cmd::Command;
    let mut cmd = Command::cargo_bin("loco-generate-scaffold-via-sql-schema").unwrap();
    cmd.write_stdin("CREATE TABLE (oops")
        .assert()
        .failure()
        .code(1);
}
```

- [ ] **Step 2: Run the integration tests**

Run: `cargo test --test snapshot`
Expected: PASS — every test passes.

If any expected output string is wrong because (for instance) `id` columns leak through or column ordering surprises us, **fix the implementation, not the test** — the expected outputs encode the spec.

- [ ] **Step 3: Run the entire suite**

Run: `cargo test`
Expected: PASS — all unit and integration tests green.

- [ ] **Step 4: Commit**

```bash
git add tests/snapshot.rs
git commit -m "test: integration snapshot tests for full pipeline"
```

---

## Task 10: README

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write README**

Create `README.md`:

```markdown
# loco-generate-scaffold-via-sql-schema

Read SQL `CREATE TABLE` statements from stdin and write equivalent
`cargo loco generate scaffold` commands to stdout.

## Install

```sh
cargo install --path .
```

## Use

```sh
cat schema.sql | loco-generate-scaffold-via-sql-schema > setup.sh
loco-generate-scaffold-via-sql-schema -d mysql -k api < schema.sql
```

## Options

- `-d, --dialect <postgres|mysql|sqlite|generic>` — default `postgres`
- `-k, --kind <htmx|html|api|none>` — default `htmx`

## Behavior

- `id`, `created_at`, `updated_at` columns are skipped (Loco generates them).
- `NOT NULL` → `!` suffix; single-column `UNIQUE` → `^` suffix; both → `!^`.
- `REFERENCES tbl(col)` becomes `<singular>:references` or
  `<singular>:references:<col>`.
- Multi-column `UNIQUE (a, b)` is dropped silently.
- `GENERATED ALWAYS AS …` columns are skipped with a warning.
- Unknown SQL types fall back to `string` with a warning.
- Tables are emitted in source order — arrange your input so FK targets
  come before referencing tables.

## Library

```rust
use loco_generate_scaffold_via_sql_schema::{convert, Options};
let (commands, warnings) = convert(sql, &Options::default())?;
```

See `docs/superpowers/specs/2026-05-04-sql-to-loco-scaffold-design.md`
for full design notes.
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: README with usage and behavior notes"
```

---

## Final Verification

- [ ] **Step 1: Full test suite green**

Run: `cargo test`
Expected: PASS — all tests.

- [ ] **Step 2: Build release**

Run: `cargo build --release`
Expected: PASS — no errors, warnings acceptable.

- [ ] **Step 3: End-to-end manual check**

Run:
```sh
cat <<'SQL' | ./target/release/loco-generate-scaffold-via-sql-schema
CREATE TABLE authors (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE
);
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    author_id INT NOT NULL REFERENCES authors(id),
    title TEXT NOT NULL,
    body TEXT NOT NULL
);
SQL
```

Expected stdout:
```
cargo loco generate scaffold authors name:text! email:text!^ --htmx

cargo loco generate scaffold posts author:references title:text! body:text! --htmx
```
