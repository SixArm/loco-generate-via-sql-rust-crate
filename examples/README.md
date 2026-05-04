# Examples

Each `*.sql` file is a runnable input. Each `*.expected` is the actual stdout
captured from running the tool against the matching `*.sql`. The expected
files double as snapshot fixtures — re-run the tool, diff the output, and
behavioral changes show up immediately.

## Index

| Example | Dialect | What it shows |
|---|---|---|
| [`blog-postgres.sql`](blog-postgres.sql) | postgres | A simple blog. Skipped boilerplate (`id`/`created_at`/`updated_at`), `!`/`^` suffixes, FK rule 1 (`author:references`). |
| [`shop-mysql.sql`](shop-mysql.sql) | mysql | E-commerce. `INT UNSIGNED` → `unsigned`, `BIGINT UNSIGNED` → `big_unsigned`, `TINYINT(1)` → `bool`, `DECIMAL(10,2)` → `decimal`. |
| [`notes-sqlite.sql`](notes-sqlite.sql) | sqlite | Notes app. `INTEGER PRIMARY KEY AUTOINCREMENT` (skipped by name), `BLOB`. |
| [`fk-rules.sql`](fk-rules.sql) | postgres | All three FK rules in one schema, side by side. |

## Running

From the repo root, after `cargo build --release`:

```sh
./target/release/loco-generate-via-sql \
    < examples/blog-postgres.sql

./target/release/loco-generate-via-sql -d mysql \
    < examples/shop-mysql.sql

./target/release/loco-generate-via-sql -d sqlite \
    < examples/notes-sqlite.sql

./target/release/loco-generate-via-sql \
    < examples/fk-rules.sql
```

Or installed:

```sh
loco-generate-via-sql < examples/blog-postgres.sql
```

## Diffing against the expected output

The shipped tool produces output byte-identical to the matching `.expected`
file. To verify — for example after editing the SQL or rebuilding the tool:

```sh
diff <(./target/release/loco-generate-via-sql \
        < examples/blog-postgres.sql) examples/blog-postgres.expected
```

A clean diff means nothing changed.

## Verifying all examples in one shot

The script [`verify.sh`](verify.sh) runs every example and reports
mismatches:

```sh
sh examples/verify.sh
```

Exits 0 if every example matches its expected file, non-zero otherwise.
