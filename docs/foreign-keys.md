# Foreign keys

Loco's scaffold has a `references` field type that creates the FK column
for you. This tool translates SQL `REFERENCES tbl(col)` clauses into one
of three forms depending on how the column is named.

## The three rules

Let `col` = the SQL column name and `tgt` = the referenced table name.
Define `singular(tgt)` = `tgt` with a single trailing `s` stripped if
present (this is intentionally naive — see [Pitfalls](#pitfalls)).

### Rule 1 — match

If `col` ends in `_id` and the prefix matches `singular(tgt)`:

```text
<prefix>:references
```

Loco creates an `<prefix>_id` column itself.

```sql
author_id INT REFERENCES authors(id)
```
```text
author:references
```

### Rule 2 — `_id` suffix, mismatched prefix

If `col` ends in `_id` but the prefix doesn't match `singular(tgt)`:

```text
<singular>:references:<col>
```

Loco creates a column named `<col>` (using your existing name).

```sql
owner_id INT REFERENCES users(id)
```
```text
user:references:owner_id
```

### Rule 3 — no `_id` suffix

```text
<singular>:references:<col>
```

Loco creates a column named `<col>`.

```sql
author INT REFERENCES users(id)
```
```text
user:references:author
```

## Worked example

The shipped fixture [`examples/fk-rules.sql`](../examples/fk-rules.sql)
demonstrates all three rules in one schema:

```sql
CREATE TABLE users   (id SERIAL PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE authors (id SERIAL PRIMARY KEY, name TEXT NOT NULL);

CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    author_id INT NOT NULL REFERENCES authors(id),  -- Rule 1
    owner_id  INT NOT NULL REFERENCES users(id),    -- Rule 2
    editor    INT NOT NULL REFERENCES users(id),    -- Rule 3
    title     TEXT NOT NULL
);
```

Output:

```text
cargo loco generate scaffold posts \
    author:references \
    user:references:owner_id \
    user:references:editor \
    title:text! \
    --htmx
```

(Wrapped here for readability; the tool emits one line per command.)

## FK columns never carry suffixes

Even if the SQL column is `NOT NULL UNIQUE`, the FK output drops the
`!`/`^` suffixes:

```sql
author_id INT NOT NULL UNIQUE REFERENCES authors(id)
```
```text
author:references
```

This is because Loco's `references` template owns the column's nullability
and uniqueness. Adding `!` would conflict with how `references` generates
the migration.

## Both inline and table-level FKs work

The tool recognizes either form:

**Inline** (column-level):
```sql
post_id INT REFERENCES posts(id)
```

**Table-level**:
```sql
post_id INT,
FOREIGN KEY (post_id) REFERENCES posts(id)
```

Both produce the same scaffold field. If both are specified, the inline FK
wins (consistent with most SQL parsers).

## What's not supported

- **Composite FKs.** `FOREIGN KEY (a, b) REFERENCES other(c, d)` is dropped
  silently. Loco's `references` field is single-column. Add the constraint
  to the migration by hand.
- **Self-referential FKs.** Technically work — `parent_id INT REFERENCES
  the_same_table(id)` follows rule 2 unless the column happens to match
  `singular(table)`. Watch the output for cycle ordering.
- **`ON DELETE CASCADE`, `ON UPDATE`, etc.** Parsed but discarded. Add
  cascade behavior to the migration after scaffolding.

## Pitfalls

### Naive depluralizer

The singularizer is "strip a trailing `s` if present". Real English
plurals are messier:

| Table name   | Singularized | Correct? |
|--------------|--------------|----------|
| `users`      | `user`       | ✓ |
| `authors`    | `author`     | ✓ |
| `posts`      | `post`       | ✓ |
| `categories` | `categorie`  | ✗ should be `category` |
| `addresses`  | `addresse`   | ✗ should be `address` |
| `analyses`   | `analyse`    | ✗ should be `analysis` |
| `bus`        | `bu`         | ✗ should be `bus` |
| `status`     | `statu`      | ✗ should be `status` |

For irregular plurals, edit the output. If you'd rather not, rename your
tables to follow the simple `<noun>s` pattern, or use Rule-2/3 spellings
explicitly (e.g. name the column to NOT match the prefix).

### Source order

The tool emits tables in input order. If table B references table A but
appears first in your input, the generated commands will fail when you
run them — Loco will try to scaffold B and reference a non-existent A.

Most pg_dump-style outputs already produce a valid order. If yours
doesn't, reorder the input file before piping.
