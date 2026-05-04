# CLI reference

The `loco-generate-via-sql` binary is a filter: it reads SQL
from stdin and writes `cargo loco generate scaffold` commands to stdout.

## Synopsis

```
loco-generate-via-sql [OPTIONS]
```

## Options

### `-d, --dialect <DIALECT>`

SQL dialect for parsing the input. Default: `postgres`.

| Value | Picks parser |
|-------|--------------|
| `postgres` | `sqlparser::dialect::PostgreSqlDialect` |
| `mysql`    | `sqlparser::dialect::MySqlDialect` |
| `sqlite`   | `sqlparser::dialect::SQLiteDialect` |
| `generic`  | `sqlparser::dialect::GenericDialect` |

The dialect affects which keywords sqlparser recognizes — for example,
`UNSIGNED` and `TINYINT(1)` need the MySQL dialect; `BYTEA`, `JSONB`, and
`SERIAL` need Postgres. `generic` is a permissive fallback.

### `-k, --kind <KIND>`

Scaffold template flag appended to each command. Default: `htmx`.

| Value | Appended flag |
|-------|---------------|
| `htmx` | `--htmx` |
| `html` | `--html` |
| `api`  | `--api` |
| `none` | (nothing — useful when piping into another tool) |

### `-h, --help`

Print help. Lists every option with its default.

### `-V, --version`

Print version (matches `Cargo.toml` `version = "..."`).

## I/O

- **stdin:** raw SQL text. Multiple statements OK; only `CREATE TABLE` is
  processed; other statements (`CREATE INDEX`, `ALTER TABLE`, comments,
  `BEGIN`/`COMMIT`) are silently skipped.
- **stdout:** generated commands. One per `CREATE TABLE`, in source order,
  separated by a blank line. The output ends with `\n`. Empty input → empty
  output.
- **stderr:** warnings prefixed `warn:` (unknown types, generated columns)
  and errors prefixed `error:` (parse failure, I/O failure).

## Exit codes

| Code | Meaning |
|------|---------|
| 0    | Success — commands written to stdout. (Warnings on stderr don't change this.) |
| 1    | SQL parse error — sqlparser couldn't read the input. |
| 2    | I/O error — failed to read stdin or write stdout. |

## Examples

**Print commands to terminal:**

```sh
loco-generate-via-sql < schema.sql
```

**Save to a runnable script:**

```sh
loco-generate-via-sql < schema.sql > setup.sh
sh setup.sh
```

**Run scaffolds inline:**

```sh
loco-generate-via-sql < schema.sql | sh -e
```

`-e` makes the shell exit if any individual scaffold fails, so you don't
silently skip past errors.

**Compare output to an expected snapshot:**

```sh
loco-generate-via-sql < schema.sql | diff - expected.txt
```

**Run with a non-default dialect and kind:**

```sh
loco-generate-via-sql -d mysql -k api < schema.sql
```

**Filter the output before running** — e.g., only run scaffolds for tables
matching a pattern:

```sh
loco-generate-via-sql < schema.sql \
    | grep '^cargo loco generate scaffold post' \
    | sh -e
```

**Suppress warnings:**

```sh
loco-generate-via-sql < schema.sql 2>/dev/null
```

**Capture warnings only:**

```sh
loco-generate-via-sql < schema.sql 2>warnings.log >/dev/null
```

## Tips

- **Run `--help`** for the canonical option list with defaults — never
  out of date.
- **Use `--kind none`** when piping into a tool that adds its own flags.
- **Source order matters** — see the [README](../README.md#source-order).
  If your schema dump puts FK targets after referencing tables, reorder
  before piping.
- **Pre-flight with `--quiet`-style stdout:** redirect stdout to
  `/dev/null` and watch stderr to see whether the input has any warnings
  before generating real output.
