# Troubleshooting

Common issues and how to fix them.

## `error: SQL parse failed: …`

`sqlparser` rejected the input as syntactically invalid. The error usually
includes the offending token and approximate position.

**Try**:

1. **Switch dialect** — `--dialect generic` is the broadest. If you were
   using `postgres` and a MySQL-flavored keyword tripped the parser, try
   `mysql` (or vice versa).
2. **Strip non-`CREATE TABLE` noise.** `pg_dump` and `mysqldump` emit
   `SET`, `BEGIN`, `COPY`, comments, etc. Most of those are skipped, but
   exotic statements occasionally fail. Try filtering:
   ```sh
   awk '/^CREATE TABLE/,/;/' schema.sql | loco-generate-via-sql
   ```
3. **Check for unbalanced parens.** A truncated dump or hand-edited file
   might have an unclosed `CREATE TABLE`. The error usually says so
   (e.g., `Expected ')' but found EOF`).
4. **Look at the specific column.** Some SQL extensions (vendor-specific
   types, computed columns with non-standard syntax) aren't recognized.
   Comment them out or simplify the offending column to confirm.

The library API surfaces this as `Err(ConvertError::Parse(msg))`.

## `warn: unknown SQL type 'X', mapped to string (column 't.c')`

`sqlparser` parsed the column but no mapping rule matched the type name.
The column was emitted as `<col>:string` (best lenient fallback).

**Fix**:

- Edit the output if `string` is wrong for the column. For example, a
  custom enum type might better be `text` or modeled as a separate table.
- File an issue if `X` is a common SQL type the table is missing — it's
  a candidate for adding to the mapping.

The library API surfaces this in the returned `Vec<Warning>`.

## `warn: skipping generated column 'tbl.col' (Loco scaffold has no equivalent)`

The column was declared `GENERATED ALWAYS AS …` (Postgres / MySQL
generated columns). Loco's scaffold has no field syntax for these.

**Fix**: After running the scaffold, edit the migration to add the
generated column manually. For Postgres:

```sql
ALTER TABLE tbl ADD COLUMN col INT GENERATED ALWAYS AS (a + 1) STORED;
```

## FK column emits as a regular field instead of `:references`

The tool only sees what `sqlparser` parses. If your column is a "logical"
FK by convention (named `_id` but with no `REFERENCES` clause), it will
be parsed as a plain integer.

**Fix**: Add an explicit `REFERENCES tbl(col)` to the schema:

```sql
-- before
post_id INT NOT NULL,

-- after
post_id INT NOT NULL REFERENCES posts(id),
```

…and re-run.

## FK output uses the wrong target name

Most likely cause: irregular pluralization. The depluralizer strips a
single trailing `s`, so:

- `categories` → `categorie` (should be `category`)
- `addresses` → `addresse` (should be `address`)
- `bus` → `bu` (should be `bus` — no `s` to strip)

**Fix**: Edit the output by hand, or rename the table to follow the
simple `<noun>s` pattern. See
[`docs/foreign-keys.md`](foreign-keys.md#pitfalls).

## Multi-column UNIQUE didn't get a `^`

By design — Loco's scaffold field syntax can't express composite
uniqueness, so `UNIQUE (a, b)` is dropped silently.

**Fix**: After scaffolding, add the constraint to the migration:

```sql
ALTER TABLE memberships ADD CONSTRAINT uq_user_group UNIQUE (user_id, group_id);
```

## Tables come out in the wrong order

The tool preserves source order. If table B references table A but B
appears first in the input, the generated commands will fail at runtime
when Loco can't find A.

**Fix**: Reorder the input. Most schema dumps (`pg_dump`, `mysqldump`)
already produce a valid order. If yours doesn't, sort by hand or
preprocess.

## Scaffold output looks right but `cargo loco generate scaffold` fails

That's a Loco issue, not this tool's — but a few common causes:

1. **Loco version mismatch.** This tool emits the type names current as of
   the design spec's revision. Older Loco versions used different names
   (`ts` instead of `tstz`, `bigint` instead of `big_int`). Check your
   Loco version's docs.
2. **Source order didn't get fixed.** Loco fails when scaffolding B before
   A, where B references A. See above.
3. **Custom names that conflict.** If your scaffold tries to create a
   `controllers/posts.rs` and one already exists, Loco refuses to
   overwrite. Delete the conflict or rename the resource.
4. **Run `cargo loco generate scaffold --help`** in your Loco project to
   confirm the exact flags your Loco version supports.

## I want to suppress all warnings

```sh
loco-generate-via-sql < schema.sql 2>/dev/null
```

…or, in library use:

```rust
let (commands, _) = convert(&sql, &opts)?;
```

(But warnings often signal real problems — inspect them at least once.)

## I want to fail on any warning

```sh
out=$(loco-generate-via-sql < schema.sql 2>warns.log)
if [ -s warns.log ]; then
    echo "schema has warnings:" >&2
    cat warns.log >&2
    exit 1
fi
echo "$out"
```

…or in library use:

```rust
let (commands, warnings) = convert(&sql, &opts)?;
if !warnings.is_empty() {
    eprintln!("schema produced {} warning(s)", warnings.len());
    for w in &warnings { eprintln!("  {}", w.message); }
    return Err("strict mode".into());
}
```

## Empty output, no error

The input contained zero `CREATE TABLE` statements. Other DDL (CREATE
INDEX, ALTER TABLE) and DML are silently skipped — only CREATE TABLE
produces commands.

Confirm with:

```sh
grep -c '^CREATE TABLE' schema.sql
```

## Still stuck?

Open an issue with:

- The exact SQL that triggered the problem (minimum reproducer ideal).
- The dialect flag you used.
- The expected output and the actual output.
- The tool version (`loco-generate-via-sql --version`).
