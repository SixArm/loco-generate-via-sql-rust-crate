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
