# Tutorial: from a schema to a working Loco app

Walk through scaffolding a small blog app from a Postgres schema. About
ten minutes start to finish.

## Prerequisites

- Rust toolchain (1.85+ for edition 2024).
- The Loco CLI: `cargo install loco`.
- A running Postgres (Loco's default; you can substitute SQLite if you'd
  rather skip Postgres setup).
- This tool installed:
  ```sh
  cargo install --path .
  ```

## 1. Start a fresh Loco project

```sh
loco new --name blog --db postgres --bg async --assets serverside
cd blog
```

Loco scaffolds the project skeleton and runs `cargo build` once.

## 2. Write the SQL schema

Save this as `schema.sql` somewhere outside the Loco project (it doesn't
need to be in the Loco repo — the schema is your input, not output):

```sql
CREATE TABLE authors (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    bio TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    author_id INT NOT NULL REFERENCES authors(id),
    title VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL UNIQUE,
    body TEXT NOT NULL,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE comments (
    id SERIAL PRIMARY KEY,
    post_id INT NOT NULL REFERENCES posts(id),
    author_id INT NOT NULL REFERENCES authors(id),
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

(This is the same schema as
[`examples/blog-postgres.sql`](../examples/blog-postgres.sql).)

## 3. Generate the scaffold commands

```sh
loco-generate-via-sql < schema.sql
```

Output:

```text
cargo loco generate scaffold authors name:text! email:text!^ bio:text --htmx

cargo loco generate scaffold posts author:references title:string! slug:string!^ body:text! published:bool! --htmx

cargo loco generate scaffold comments post:references author:references body:text! --htmx
```

Three scaffold commands, blank-line separated, one per `CREATE TABLE`.
Note:

- `id`, `created_at`, `updated_at` columns are gone — Loco generates them.
- `email TEXT NOT NULL UNIQUE` became `email:text!^`.
- `author_id INT REFERENCES authors(id)` became `author:references` —
  Loco will create an `author_id` column wired up to `authors`.
- `--htmx` makes Loco scaffold HTMX views (use `-k html` or `-k api` for
  alternatives).

## 4. Save and run the commands

Pipe into a script you can review before running:

```sh
loco-generate-via-sql < schema.sql > setup.sh
cat setup.sh   # eyeball it
```

Then run from the Loco project root:

```sh
sh setup.sh
```

Each `cargo loco generate scaffold` call creates:

- A SeaORM model in `src/models/`.
- A controller in `src/controllers/`.
- HTMX views in `assets/views/`.
- A migration in `migration/src/`.
- Routes registered in `src/app.rs`.

Or run the scaffolds inline without an intermediate file:

```sh
loco-generate-via-sql < schema.sql | sh -e
```

The `-e` ensures any individual scaffold failure aborts the whole run.

## 5. Apply migrations

```sh
cargo loco db migrate
```

Migrations run in the order Loco generated them, which matches the order
the scaffolds ran in, which matches the source order from your schema.

## 6. Run the server

```sh
cargo loco start
```

Visit `http://localhost:5150/posts`. You'll see the HTMX scaffold UI for
the `posts` resource — list, create, edit, delete.

## 7. Customize

The scaffold gives you a working baseline. From here:

- **Adjust column types.** A `slug` shown as `string` might really want
  `string!^` (already there) plus a custom validator in the model.
- **Add custom routes.** Edit `src/app.rs` after the scaffold-registered
  routes.
- **Tweak views.** The HTMX templates live in `assets/views/<resource>/`.
- **Add custom validation.** Edit the SeaORM model in `src/models/`.
- **Wire up associations.** The FK columns are physical; the SeaORM
  associations may need to be added by hand for richer queries.

## What this tool doesn't do

- It doesn't run `loco new` for you — you start with a Loco project.
- It doesn't apply migrations — `cargo loco db migrate` does that.
- It doesn't preserve your edits if you re-run the scaffolds. Loco's
  generators overwrite by default; commit your work to git before
  regenerating.

## Variations

### Use HTML scaffolds instead of HTMX

```sh
loco-generate-via-sql -k html < schema.sql | sh -e
```

### Use API-only scaffolds

```sh
loco-generate-via-sql -k api < schema.sql | sh -e
```

### Use SQLite instead of Postgres

Re-create the Loco project with `--db sqlite`, then write your schema
using SQLite syntax (`INTEGER PRIMARY KEY AUTOINCREMENT` instead of
`SERIAL PRIMARY KEY`, etc.) and pass `-d sqlite`:

```sh
loco-generate-via-sql -d sqlite < schema.sql | sh -e
```

## See also

- [`examples/`](../examples/) — more SQL fixtures with expected output.
- [`docs/foreign-keys.md`](foreign-keys.md) — the three FK rules in
  detail.
- [`docs/types.md`](types.md) — full SQL → Loco type table.
- [`docs/dialects.md`](dialects.md) — picking the right `--dialect`.
- [`docs/troubleshooting.md`](troubleshooting.md) — common issues.
