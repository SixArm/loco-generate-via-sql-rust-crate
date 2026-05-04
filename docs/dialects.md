# Dialects

Pick `--dialect` to match your source SQL. The four choices select different
parsers in `sqlparser`, which differ in which dialect-specific keywords they
recognize.

## When in doubt: `generic`

`--dialect generic` accepts the broadest grammar. Try it first if you're
parsing a hand-rolled or mixed-dialect schema. Fall back to a specific
dialect only if generic mis-parses something.

## `postgres`

```sh
loco-generate-via-sql -d postgres < schema.sql
```

Recognizes:

- `SERIAL`, `BIGSERIAL` (auto-increment integer types) — both map to
  `int`/`big_int` in Loco. The auto-increment is implicit because Loco
  scaffolds always include `id` as the primary key, and we skip `id`
  columns by name.
- `UUID` → `uuid`.
- `JSONB` → `jsonb`. Plain `JSON` → `json`.
- `BYTEA` → `blob`.
- `TIMESTAMPTZ` and `TIMESTAMP WITH TIME ZONE` → `tstz`.
- `MONEY` → `money`.
- Schema-qualified names: `public.users` → `users` (last segment used).
- Postgres array types `int[]`, `text[]` → `array` (element type dropped).
- Double-quoted identifiers: `"User"`.

Common Postgres patterns that work well:

```sql
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    author_id INT NOT NULL REFERENCES authors(id),
    body TEXT NOT NULL,
    metadata JSONB,
    tags TEXT[],
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## `mysql`

```sh
loco-generate-via-sql -d mysql < schema.sql
```

Recognizes:

- `UNSIGNED` modifier on integer types — `INT UNSIGNED` → `unsigned`,
  `BIGINT UNSIGNED` → `big_unsigned`, `SMALLINT UNSIGNED` →
  `small_unsigned`.
- `TINYINT(1)` → `bool` (MySQL convention for boolean storage).
- `TINYINT` (no `(1)`) → `small_int`.
- `MEDIUMINT` → `small_int` (24-bit truncated to 16-bit — see
  [`types.md`](types.md#mediumint-truncation)).
- `MEDIUMTEXT`, `LONGTEXT` → `text`.
- `MEDIUMBLOB`, `LONGBLOB` → `blob`.
- Backtick-quoted identifiers: `` `User` ``.
- `DATETIME` → `date_time`.
- `DECIMAL(p,s)` → `decimal` (the `(p,s)` is parsed but discarded).

Common MySQL patterns:

```sql
CREATE TABLE products (
    id INT UNSIGNED NOT NULL PRIMARY KEY,
    sku VARCHAR(64) NOT NULL UNIQUE,
    price DECIMAL(10, 2) NOT NULL,
    stock INT UNSIGNED NOT NULL DEFAULT 0,
    active TINYINT(1) NOT NULL DEFAULT 1,
    notes MEDIUMTEXT
);
```

## `sqlite`

```sh
loco-generate-via-sql -d sqlite < schema.sql
```

Recognizes:

- `INTEGER PRIMARY KEY AUTOINCREMENT` — the `AUTOINCREMENT` keyword
  parses cleanly here.
- SQLite "type affinity" — types like `BLOB`, `TEXT`, `INTEGER`, `REAL`,
  `NUMERIC` work as expected.
- Bracket-quoted identifiers: `[User]`.

Common SQLite patterns:

```sql
CREATE TABLE notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    notebook_id INTEGER NOT NULL REFERENCES notebooks(id),
    title TEXT NOT NULL,
    body TEXT,
    pinned INTEGER NOT NULL DEFAULT 0
);
```

Note: SQLite uses `INTEGER` as the storage class for booleans by
convention. This tool maps `INTEGER` to `int`, not `bool`. If a column is
semantically a boolean, you'll need to either (a) edit the output,
(b) rename the column type to `BOOLEAN` (which sqlparser accepts in
SQLite mode), or (c) accept that the scaffold creates an int column
backed by a 0/1.

## `generic`

```sh
loco-generate-via-sql -d generic < schema.sql
```

Permissive grammar — accepts most syntax from any of the dialects above.
Useful when:

- The SQL was written without a specific dialect in mind.
- You're not sure which dialect a particular schema dump came from.
- You want to be lenient about parser quirks (rare).

Trade-off: dialect-specific keywords like `UNSIGNED` may not parse the
same way as in their native dialect. If a `--dialect generic` run
produces unexpected output for a known dialect feature, switch to the
specific dialect.

## Mixing dialects

The tool runs one parser per invocation. If your input mixes Postgres and
MySQL conventions, neither dialect will parse it cleanly. Two options:

1. **Split the schema** into per-dialect files and run the tool once per
   file:
   ```sh
   loco-generate-via-sql -d postgres < pg-tables.sql > pg.sh
   loco-generate-via-sql -d mysql    < my-tables.sql > my.sh
   cat pg.sh my.sh > setup.sh
   ```

2. **Try `--dialect generic`** and review the output. Generic accepts most
   keywords from all four backends.

## Picking a dialect from a `pg_dump` / `mysqldump`

- `pg_dump` → `--dialect postgres`. Strip the `SET …;` and other Postgres
  metadata at the top of the file before piping; the tool ignores them
  but it's cleaner.
- `mysqldump` → `--dialect mysql`. Strip the leading `/*!40101 …;` directives
  and any backtick-heavy `DROP TABLE IF EXISTS` if they confuse the parser.
- `sqlite3 .schema` → `--dialect sqlite`. The output is usually clean SQL
  already.

## Sanity check

After picking a dialect, do a quick:

```sh
your-schema-dump | loco-generate-via-sql -d <dialect> 2>warns.log >cmds.sh
```

…and inspect both `cmds.sh` (commands) and `warns.log` (any unknown types
or skipped generated columns) before running the scaffolds.
