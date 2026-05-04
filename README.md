# loco-generate-scaffold-via-sql-schema

Read SQL `CREATE TABLE` statements from stdin and write equivalent
`cargo loco generate scaffold` commands to stdout — one per table, blank-line
separated, ready to pipe into a shell.

```sh
$ echo 'CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);' \
    | loco-generate-scaffold-via-sql-schema
cargo loco generate scaffold posts title:text! --htmx
```

The tool is a small filter: stdin → stdout, no side effects. Pipe its output
into `bash` to actually run the scaffolds, redirect to a file to commit a
build script, or eyeball it before running.

---

## Table of contents

- [Why](#why)
- [Install](#install)
- [Quick start](#quick-start)
- [CLI reference](#cli-reference)
- [Behavior](#behavior)
  - [Skipped columns](#skipped-columns)
  - [Suffixes](#suffixes)
  - [Foreign keys](#foreign-keys)
  - [Type mapping](#type-mapping)
  - [Multi-column constraints](#multi-column-constraints)
  - [Generated columns](#generated-columns)
  - [Schema-qualified names](#schema-qualified-names)
  - [Source order](#source-order)
- [Dialects](#dialects)
- [Library use](#library-use)
- [Exit codes and warnings](#exit-codes-and-warnings)
- [Examples](#examples)
- [Limitations](#limitations)
- [Troubleshooting](#troubleshooting)
- [Documentation](#documentation)
- [License](#license)

---

## Why

[Loco](https://loco.rs) is a Rails-like Rust web framework. New apps are
typically built up one resource at a time with `cargo loco generate scaffold
<table> <field:type>… <--htmx|--html|--api>`. For a database with more than a
handful of tables, hand-typing those commands — translating SQL types to Loco
short names, working out which columns are foreign keys, remembering the `!`
and `^` suffix conventions — is tedious and error-prone.

This tool reads the SQL schema you already have and produces the exact
scaffold commands. You stay in control: review the output, edit it, run it
when you're ready.

## Install

From a clone:

```sh
cargo install --path .
```

The binary is installed to `~/.cargo/bin/loco-generate-scaffold-via-sql-schema`.

## Quick start

**1. Pipe a schema in, see the commands:**

```sh
cat schema.sql | loco-generate-scaffold-via-sql-schema
```

**2. Save to a runnable script:**

```sh
cat schema.sql | loco-generate-scaffold-via-sql-schema > setup.sh
chmod +x setup.sh
./setup.sh
```

**3. Run directly:**

```sh
cat schema.sql | loco-generate-scaffold-via-sql-schema | sh -e
```

**4. Different dialect or scaffold kind:**

```sh
cat schema.sql | loco-generate-scaffold-via-sql-schema -d mysql -k api
```

## CLI reference

```
loco-generate-scaffold-via-sql-schema [OPTIONS]

OPTIONS:
  -d, --dialect <DIALECT>  SQL dialect: postgres | mysql | sqlite | generic
                           [default: postgres]
  -k, --kind <KIND>        Scaffold template: htmx | html | api | none
                           [default: htmx]
  -h, --help               Print help
  -V, --version            Print version
```

Reads SQL from stdin and writes commands to stdout. Warnings (unknown types,
generated columns) go to stderr with a `warn:` prefix. Errors go to stderr
with an `error:` prefix.

## Behavior

### Skipped columns

Columns named `id`, `created_at`, or `updated_at` are skipped — Loco's
scaffold generator creates those automatically.

```sql
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);
```

```text
cargo loco generate scaffold posts title:text! --htmx
```

### Suffixes

| SQL                 | Loco field |
|---------------------|------------|
| `name TEXT`         | `name:text` |
| `name TEXT NOT NULL`| `name:text!` |
| `name TEXT UNIQUE`  | `name:text^` |
| `name TEXT NOT NULL UNIQUE` | `name:text!^` |

`PRIMARY KEY` on a non-`id` column also implies `NOT NULL` and emits `!`.

### Foreign keys

Three rules decide how `REFERENCES tbl(col)` is rendered. The shape depends
on whether the column name ends in `_id` and whether its prefix matches the
target table's depluralized name.

**Rule 1 — match.** Column ends in `_id` AND prefix matches the singularized
target table → `<prefix>:references` (Loco creates the `<prefix>_id` column
itself).

```sql
author_id INT REFERENCES authors(id)
```
```text
author:references
```

**Rule 2 — `_id` suffix, mismatched prefix.** → `<singular>:references:<col>`.

```sql
owner_id INT REFERENCES users(id)
```
```text
user:references:owner_id
```

**Rule 3 — no `_id` suffix.** → `<singular>:references:<col>`.

```sql
author INT REFERENCES users(id)
```
```text
user:references:author
```

FK columns never carry `!` or `^` — Loco's `references` template owns the
nullability and uniqueness for those columns. A `NOT NULL UNIQUE` foreign-key
column still emits the bare references form.

> **Heads up:** the depluralizer is naive (strips a trailing `s`).
> `categories → categorie`, `bus → bu`, `analyses → analyse` will be wrong.
> Rename the target table or accept that you'll edit the output.

### Type mapping

The mapping is dialect-aware: sqlparser parses dialect-specific keywords,
then the result is normalized to one of Loco's canonical type names.
Length/precision arguments are parsed but discarded for type selection
(Loco's type names don't carry them).

| SQL type                                         | Loco type     |
|--------------------------------------------------|---------------|
| `UUID`                                           | `uuid`        |
| `BOOL`, `BOOLEAN`, `TINYINT(1)`                  | `bool`        |
| `SMALLINT`, `INT2`, `TINYINT`, `MEDIUMINT`       | `small_int`   |
| `INT`, `INT4`, `INTEGER`, `SERIAL`               | `int`         |
| `BIGINT`, `INT8`, `BIGSERIAL`                    | `big_int`     |
| `SMALLINT UNSIGNED`                              | `small_unsigned` |
| `INT UNSIGNED`                                   | `unsigned`    |
| `BIGINT UNSIGNED`                                | `big_unsigned`|
| `REAL`, `FLOAT4`                                 | `float`       |
| `DOUBLE`, `DOUBLE PRECISION`, `FLOAT8`, `FLOAT`  | `double`      |
| `NUMERIC`, `DECIMAL`                             | `decimal`     |
| `MONEY`                                          | `money`       |
| `CHAR`, `VARCHAR`, `NVARCHAR`, `CHARACTER VARYING` | `string`    |
| `TEXT`, `MEDIUMTEXT`, `LONGTEXT`, `CLOB`         | `text`        |
| `DATE`                                           | `date`        |
| `TIME`                                           | `string` (no Loco equivalent) |
| `TIMESTAMP`, `DATETIME`                          | `date_time`   |
| `TIMESTAMPTZ`, `TIMESTAMP WITH TIME ZONE`        | `tstz`        |
| `JSON`                                           | `json`        |
| `JSONB`                                          | `jsonb`       |
| `<type>[]`, `ARRAY`                              | `array`       |
| `BYTEA`, `BLOB`, `MEDIUMBLOB`, `LONGBLOB`        | `blob`        |
| `BINARY(n)`                                      | `binary_len`  |
| `VARBINARY(n)`                                   | `var_binary`  |
| anything else                                    | `string` (with a warning) |

See [`docs/types.md`](docs/types.md) for notes on edge cases (`MEDIUMINT`
truncation, `FLOAT(p)` precision rules, why some fall back to `string`).

### Multi-column constraints

`UNIQUE (a, b)` is dropped silently — Loco's scaffold field syntax can't
express composite uniqueness. If you need it, add it to your migration by
hand after running the scaffold.

`FOREIGN KEY (a, b) REFERENCES other(c, d)` is also dropped — Loco's
`references` field is single-column.

### Generated columns

`GENERATED ALWAYS AS (…) STORED` and `GENERATED BY DEFAULT AS …` columns are
skipped with a stderr warning. Loco's scaffold has no equivalent; add the
generated column to the migration manually.

### Schema-qualified names

`public.users` becomes `users`. The schema prefix is stripped — Loco doesn't
namespace tables in scaffold names.

### Source order

Tables are emitted in input order. The tool does not topologically sort by
foreign-key dependency. Arrange your input so target tables come before
referencing tables, or you'll see Loco migration failures when running the
generated commands. Most schema dumps already produce a valid order.

## Dialects

Pick the dialect that matches your source SQL. The differences are the
keywords each dialect's parser recognizes.

| `--dialect` | sqlparser dialect | Recognizes |
|-------------|-------------------|-----------|
| `postgres`  | `PostgreSqlDialect` | `SERIAL`, `BIGSERIAL`, `UUID`, `JSONB`, `BYTEA`, `TIMESTAMPTZ`, schema-qualified names, `[]` arrays |
| `mysql`     | `MySqlDialect`     | `UNSIGNED` modifier, `TINYINT(1)`, `MEDIUMINT`, `MEDIUMBLOB`, `LONGBLOB`, backtick-quoted identifiers |
| `sqlite`    | `SQLiteDialect`    | `AUTOINCREMENT`, integer affinity, bracket-quoted identifiers |
| `generic`   | `GenericDialect`   | Broad fallback grammar — accepts most syntax from the others |

When in doubt, try `--dialect generic`. See [`docs/dialects.md`](docs/dialects.md)
for dialect-specific notes.

## Library use

The conversion is also a library function. Add to `Cargo.toml`:

```toml
[dependencies]
loco-generate-scaffold-via-sql-schema = "0.1"
```

```rust
use loco_generate_scaffold_via_sql_schema::{convert, Dialect, Options, ScaffoldKind};

let sql = std::fs::read_to_string("schema.sql")?;
let opts = Options { dialect: Dialect::Postgres, kind: ScaffoldKind::Htmx };
let (commands, warnings) = convert(&sql, &opts)?;

for w in &warnings {
    eprintln!("warn: {}", w.message);
}
println!("{commands}");
# Ok::<_, Box<dyn std::error::Error>>(())
```

There's also `convert_to_writer` for streaming directly into any
[`std::io::Write`]:

```rust
use loco_generate_scaffold_via_sql_schema::{convert_to_writer, Options};
use std::io::stdout;

let sql = "CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);";
let warnings = convert_to_writer(sql, &Options::default(), &mut stdout().lock())?;
# Ok::<_, Box<dyn std::error::Error>>(())
```

Run `cargo doc --open --no-deps` for full API docs with runnable examples.
See also [`docs/library.md`](docs/library.md).

## Exit codes and warnings

| Exit | Meaning |
|------|---------|
| 0    | Success — commands written to stdout (may also have warnings on stderr). |
| 1    | SQL parse error — sqlparser couldn't read the input. |
| 2    | I/O error — failed to read stdin or write stdout. |

Warnings go to stderr with a `warn:` prefix; they don't change the exit code.
Errors go to stderr with an `error:` prefix.

## Examples

The [`examples/`](examples/) directory has runnable fixtures with their
expected outputs:

- [`blog-postgres.sql`](examples/blog-postgres.sql) — a small blog schema
  exercising FK rule 1 and `!^` suffixes.
- [`shop-mysql.sql`](examples/shop-mysql.sql) — MySQL `UNSIGNED`,
  `TINYINT(1)` boolean, decimal money.
- [`notes-sqlite.sql`](examples/notes-sqlite.sql) — SQLite
  `INTEGER PRIMARY KEY AUTOINCREMENT`.
- [`fk-rules.sql`](examples/fk-rules.sql) — all three FK rules in one schema.

Run any of them:

```sh
loco-generate-scaffold-via-sql-schema < examples/blog-postgres.sql
```

…and compare to `examples/blog-postgres.expected`.

## Limitations

- **No topological sort.** Tables are emitted in source order. If your
  schema dumps tables out of FK order, reorder the input.
- **Single-column references only.** Composite foreign keys are dropped.
- **Naive depluralizer.** `categories → categorie`, `analyses → analyse`,
  `bus → bu`. Rename the target table or edit the output.
- **Loco type names are version-current.** This tool emits `tstz`,
  `big_int`, `date_time`, `decimal`, etc. — names current in Loco's docs at
  the time of writing. Older Loco versions used `ts`, `bigint`, etc.
- **No schema rewrites.** The tool reads SQL and writes scaffold commands.
  It doesn't generate Loco migrations directly, doesn't run them, and doesn't
  modify your project.

## Troubleshooting

**"error: SQL parse failed: …"** — sqlparser rejected the input. Try
`--dialect generic` if you're parsing mixed-dialect SQL. See
[`docs/troubleshooting.md`](docs/troubleshooting.md).

**"warn: unknown SQL type 'X', mapped to string"** — sqlparser parsed your
column but no mapping rule matched the type name. The column was emitted as
`<col>:string` (best fallback). Edit the output if `string` is wrong, and/or
file an issue if a common SQL type is missing from the table.

**FK column emits as a regular field instead of `:references`** — make sure
the `REFERENCES` clause is in the SQL. The tool only sees what sqlparser
parses; if the FK is implicit (column name convention only), add an explicit
`REFERENCES` to the schema.

**Output isn't a valid shell script** — that's expected for `--kind none` if
you intended a runnable script. Use the default `--kind htmx` (or `html`/`api`)
for a complete command.

## Documentation

- [`docs/cli.md`](docs/cli.md) — CLI reference, exit codes, examples.
- [`docs/library.md`](docs/library.md) — Rust library API guide.
- [`docs/types.md`](docs/types.md) — full type-mapping reference + edge cases.
- [`docs/foreign-keys.md`](docs/foreign-keys.md) — FK rules, worked examples,
  pitfalls.
- [`docs/dialects.md`](docs/dialects.md) — dialect-specific notes.
- [`docs/troubleshooting.md`](docs/troubleshooting.md) — common errors.
- [`docs/tutorial.md`](docs/tutorial.md) — step-by-step "blog app" walkthrough.
- [`docs/superpowers/specs/2026-05-04-sql-to-loco-scaffold-design.md`](docs/superpowers/specs/2026-05-04-sql-to-loco-scaffold-design.md)
  — full design notes.

## License

MIT OR Apache-2.0
