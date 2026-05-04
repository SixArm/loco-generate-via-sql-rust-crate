# SQL Schema → `cargo loco generate scaffold` — Design

## Purpose

A Rust crate that reads SQL `CREATE TABLE` statements from stdin and writes
equivalent `cargo loco generate scaffold` commands to stdout, one per table.
It replaces the regex-based Python script
`generate-full-stack-with-loco-tera-htmx-alpine-setup.py` with a typed,
dialect-aware, library-shaped Rust tool.

## Scope

In scope:
- Parse `CREATE TABLE` statements (Postgres, MySQL, SQLite, generic SQL).
- Map SQL column types to current Loco scaffold type names.
- Emit one `cargo loco generate scaffold <table> <fields...> [--kind]` line
  per table, in source order, separated by blank lines.
- Library + thin binary. Stdin → stdout. Configurable via two CLI flags.

Out of scope:
- The Python script's `setup.sh` shell-script wrapper (header, `set -euf`,
  `&&` chaining, `echo` lines).
- Domain-specific table-merge behavior (e.g., the Python's
  `assessment_<section>` folding).
- Index/constraint/trigger/view emission.
- Loco project scaffolding, migration execution, or any side effects beyond
  writing to stdout.

## Architecture

```
loco-generate-scaffold-via-sql-schema/
├── Cargo.toml              # edition 2024, deps: sqlparser, clap (derive), thiserror
├── src/
│   ├── lib.rs              # public API + Options
│   ├── parse.rs            # SQL text → Vec<TableDef>  (uses sqlparser)
│   ├── ir.rs               # TableDef, ColumnDef, FieldKind, Suffix flags
│   ├── types.rs            # SQL data type → Loco type name
│   ├── emit.rs             # &TableDef + &Options → String
│   └── main.rs             # CLI: stdin → stdout
└── tests/
    └── snapshot.rs         # whole-pipeline tests
```

Pipeline of pure functions, with a small IR between the parser and the emitter
so each stage is independently testable.

### Public library API

```rust
pub struct Options {
    pub dialect: Dialect,    // Postgres | MySql | SQLite | Generic
    pub kind: ScaffoldKind,  // Htmx | Html | Api | None
}
pub enum Dialect { Postgres, MySql, SQLite, Generic }
pub enum ScaffoldKind { Htmx, Html, Api, None }

pub fn convert(sql: &str, opts: &Options) -> Result<String, ConvertError>;
pub fn convert_to_writer<W: std::io::Write>(
    sql: &str, opts: &Options, w: &mut W,
) -> Result<(), ConvertError>;
```

### Data flow

stdin → `convert` → `parse::parse_sql` → `Vec<TableDef>` → for each table,
`emit::emit_command` → push to output, separated by blank lines → stdout.

Tables are emitted in source order. Callers are responsible for arranging
their input so FK targets appear before referencing tables — the tool does
not topologically sort.

## Type mapping (SQL → Loco)

Mapping is dialect-aware (sqlparser parses dialect-specific keywords) but
the output uses one canonical Loco name. Unrecognized SQL types fall back
to `string` (lenient — never crashes on stray dialects).

| SQL type | Loco type |
|---|---|
| `UUID` | `uuid` |
| `BOOL`, `BOOLEAN`, `TINYINT(1)` | `bool` |
| `SMALLINT`, `INT2`, `TINYINT` (non-(1)), `MEDIUMINT` | `small_int` |
| `INT`, `INT4`, `INTEGER`, `SERIAL` | `int` |
| `BIGINT`, `INT8`, `BIGSERIAL` | `big_int` |
| `SMALLINT UNSIGNED` | `small_unsigned` |
| `INT UNSIGNED` | `unsigned` |
| `BIGINT UNSIGNED` | `big_unsigned` |
| `REAL`, `FLOAT4` | `float` |
| `DOUBLE`, `DOUBLE PRECISION`, `FLOAT8` | `double` |
| `FLOAT` (no precision) | `double` |
| `NUMERIC(p,s)`, `DECIMAL(p,s)` | `decimal` |
| `MONEY` | `money` |
| `CHAR`, `CHARACTER`, `VARCHAR`, `NVARCHAR`, `CHARACTER VARYING` | `string` |
| `TEXT`, `MEDIUMTEXT`, `LONGTEXT`, `CLOB` | `text` |
| `DATE` | `date` |
| `TIME` | `string` (Loco has no `time` per docs — fall back) |
| `TIMESTAMP`, `DATETIME` | `date_time` |
| `TIMESTAMPTZ`, `TIMESTAMP WITH TIME ZONE` | `tstz` |
| `JSON` | `json` |
| `JSONB` | `jsonb` |
| `ARRAY`, `<type>[]` | `array` |
| `BYTEA`, `BLOB`, `MEDIUMBLOB`, `LONGBLOB` | `blob` |
| `BINARY(n)` | `binary_len` |
| `VARBINARY(n)` | `var_binary` |
| anything else | `string` (with a stderr warning) |

Notes:
- Length/precision parameters (`VARCHAR(255)`, `NUMERIC(10,2)`) are parsed
  but discarded for type selection — Loco's type names don't carry them.
- MySQL `UNSIGNED` modifier is detected from sqlparser's column data type
  and rewrites `int → unsigned`, `small_int → small_unsigned`,
  `big_int → big_unsigned`.
- `TINYINT(1)` → `bool` (MySQL convention). Bare `TINYINT` → `small_int`.

## Field-spec emission rules

### Skipped columns

- Column name in `{ id, created_at, updated_at }` — skip entirely (Loco
  generates these automatically).
- Table-level constraints (`PRIMARY KEY`, `FOREIGN KEY`, `UNIQUE`, `CHECK`,
  `CONSTRAINT …`) — never emitted as fields. sqlparser already separates
  these from `ColumnDef`s.
- `GENERATED ALWAYS AS …` columns — skipped with a stderr warning.

### Suffixes

In this order, no separator:

- `!` if the column is `NOT NULL` (or has `PRIMARY KEY` inline, which
  implies NOT NULL).
- `^` if the column has an inline `UNIQUE` constraint **or** is the sole
  member of a single-column table-level `UNIQUE(col)` constraint.
  Multi-column `UNIQUE(a, b)` is dropped silently (no warning) — Loco's
  scaffold field syntax cannot express composite uniqueness, and it would
  be too noisy to warn on every composite unique index.

A `NOT NULL UNIQUE` column emits `name:type!^`.

### Foreign keys

A column has a foreign key when it carries `REFERENCES <table>(<col>)`
either inline or in a table-level `FOREIGN KEY (col) REFERENCES …` clause.

Let `col` = SQL column name, `tgt` = referenced table.

1. If `col` ends in `_id` and the prefix matches `tgt` (singular/plural
   insensitive — strip trailing `s`): emit `<prefix>:references`.
   `author_id REFERENCES authors(id)` → `author:references` (Loco creates
   the column `author_id`).
2. If `col` ends in `_id` but the prefix doesn't match `tgt`: emit
   `<tgt_singular>:references:<col>`.
   `owner_id REFERENCES users(id)` → `user:references:owner_id`.
3. If `col` does not end in `_id`: emit
   `<tgt_singular>:references:<col>`.
   `author REFERENCES users(id)` → `user:references:author`.

`<tgt_singular>` is a naive depluralizer: strip a trailing `s` if present,
else use `tgt` as-is.

FK-detected fields **never** carry `!`/`^` suffixes — Loco's `references`
template owns nullability/uniqueness for those columns.

### Identifier handling

- Quoted identifiers (`"User"`, `` `user` ``, `[user]`) are unwrapped to
  bare identifiers.
- Schema-qualified table names (`public.users`) use the last segment
  (`users`) for the scaffold resource name.

### Final emission per table

```
cargo loco generate scaffold <table> <field1> <field2> … [--htmx|--html|--api]
```

- One command per line (no `\` continuations).
- Blank line between commands.
- Kind flag (`--htmx`/`--html`/`--api`) appended unless `kind = None`.

## CLI surface

```
loco-generate-scaffold-via-sql-schema [OPTIONS]

Reads SQL from stdin and writes `cargo loco generate scaffold` commands to stdout.

OPTIONS:
  -d, --dialect <DIALECT>   SQL dialect: postgres | mysql | sqlite | generic
                            [default: postgres]
  -k, --kind <KIND>         Scaffold template: htmx | html | api | none
                            [default: htmx]
  -h, --help                Print help
  -V, --version             Print version
```

Examples:

```sh
cat schema.sql | loco-generate-scaffold-via-sql-schema > setup-commands.sh
loco-generate-scaffold-via-sql-schema -d sqlite -k api < schema.sql
```

`clap` with derive macros. No subcommands — the tool does one thing.
Binary name is the crate name.

## Error handling

| Failure | Behavior |
|---|---|
| stdin read error | print `error: <msg>` to stderr, exit 2 |
| sqlparser fatal parse error | print `error: parse failed at line N: <msg>` to stderr, exit 1 |
| Unknown SQL type → fallback to `string` | stderr warning (`warn: unknown type 'FOO' on table.col, mapped to string`), exit 0 |
| Generated/computed column skipped | stderr warning, exit 0 |
| Input contains zero `CREATE TABLE` statements | exit 0 with empty stdout |

`thiserror` for `ConvertError`. The library never panics; the binary is
the only place that calls `process::exit`.

## Testing strategy

Three layers:

1. **Unit tests** (in-module `#[cfg(test)]`):
   - `types.rs`: table-driven mapping tests for every entry in the type
     table above.
   - `emit.rs`: per-table emission for each FK rule (1/2/3), every suffix
     combination, skipped columns, every kind variant.
2. **Integration tests** (`tests/snapshot.rs`): SQL fixture in → exact
   stdout out. Fixtures cover:
   - Simple Postgres table.
   - FK chain across multiple tables (preserves source order).
   - MySQL `UNSIGNED` and `TINYINT(1)`.
   - SQLite `AUTOINCREMENT` and integer affinity.
   - Multi-column `UNIQUE` (must not produce `^`).
   - Generated column (must be skipped, must warn).
   - Schema-qualified table name (`public.users`).
3. **Binary smoke test**: spawn the binary with `assert_cmd`, pipe stdin,
   assert stdout exactly. One test per kind flag value.

No mocking. sqlparser is the real parser; tests run against real SQL strings.

## Dependencies

| Crate | Purpose |
|---|---|
| `sqlparser` | SQL AST parser (dialect-aware) |
| `clap` (derive) | CLI argument parsing |
| `thiserror` | `ConvertError` error type |
| `assert_cmd` (dev) | Binary smoke tests |

No async runtime. No serde. No regex. Build-time and binary size kept
minimal beyond what sqlparser itself brings.
