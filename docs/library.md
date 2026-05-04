# Library use

The conversion is exposed as a Rust library — same logic the CLI uses.
Useful when you want to embed scaffold generation in a larger build script,
a custom developer tool, or an editor integration.

## Add to your `Cargo.toml`

```toml
[dependencies]
loco-generate-via-sql = "0.1"
```

## Public API

The whole surface is small:

```rust
pub fn convert(sql: &str, opts: &Options) -> Result<(String, Vec<Warning>), ConvertError>;

pub fn convert_to_writer<W: std::io::Write>(
    sql: &str, opts: &Options, w: &mut W,
) -> Result<Vec<Warning>, ConvertError>;

pub struct Options { pub dialect: Dialect, pub kind: ScaffoldKind }
pub enum Dialect { Postgres, MySql, SQLite, Generic }
pub enum ScaffoldKind { Htmx, Html, Api, None }
pub struct Warning { pub message: String }
pub enum ConvertError { Parse(String), Io(std::io::Error) }
```

`Options::default()` returns `(Postgres, Htmx)`.

## Common patterns

### Parse and print

```rust
use loco_generate_via_sql::{convert, Options};

let sql = std::fs::read_to_string("schema.sql")?;
let (commands, warnings) = convert(&sql, &Options::default())?;

for w in &warnings {
    eprintln!("warn: {}", w.message);
}
print!("{commands}");
# Ok::<_, Box<dyn std::error::Error>>(())
```

### Stream into an arbitrary writer

```rust
use loco_generate_via_sql::{convert_to_writer, Options};
use std::fs::File;
use std::io::BufWriter;

let sql = "CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);";
let file = File::create("setup.sh")?;
let mut writer = BufWriter::new(file);
let warnings = convert_to_writer(sql, &Options::default(), &mut writer)?;
# Ok::<_, Box<dyn std::error::Error>>(())
```

### Pick dialect and kind explicitly

```rust
use loco_generate_via_sql::{convert, Dialect, Options, ScaffoldKind};

let opts = Options { dialect: Dialect::MySql, kind: ScaffoldKind::Api };
let sql = "CREATE TABLE widgets (id INT UNSIGNED NOT NULL, qty INT UNSIGNED NOT NULL);";
let (out, _) = convert(sql, &opts)?;
assert_eq!(out, "cargo loco generate scaffold widgets qty:unsigned! --api\n");
# Ok::<_, Box<dyn std::error::Error>>(())
```

### Ignore warnings

```rust
use loco_generate_via_sql::{convert, Options};

let sql = "CREATE TABLE x (q WIDGET);";
let commands = convert(sql, &Options::default())?.0;  // .0 = String, .1 = Vec<Warning>
# Ok::<_, Box<dyn std::error::Error>>(())
```

### Treat warnings as failures

```rust
use loco_generate_via_sql::{convert, Options};

let sql = "CREATE TABLE x (q WIDGET);";
let (commands, warnings) = convert(sql, &Options::default())?;
if !warnings.is_empty() {
    return Err("schema produced warnings — fix and retry".into());
}
# Ok::<_, Box<dyn std::error::Error>>(())
```

### Match on errors

```rust
use loco_generate_via_sql::{convert, ConvertError, Options};

match convert("CREATE TABLE (oops", &Options::default()) {
    Ok((out, _)) => print!("{out}"),
    Err(ConvertError::Parse(msg)) => eprintln!("SQL parse failed: {msg}"),
    Err(ConvertError::Io(e)) => eprintln!("I/O failure: {e}"),
}
```

## Behavior contract

- **Never panics.** Every fatal failure surfaces as `Err(ConvertError)`.
- **Never writes to stderr.** Warnings are returned in the `Vec<Warning>`;
  callers decide whether to log them.
- **Output format** is the same as the CLI's stdout: command per `CREATE
  TABLE`, blank-line separated, trailing `\n`, empty input → empty string.
- **Tables are emitted in source order.** No topological sort.

## Build a tiny in-process REPL

```rust
use loco_generate_via_sql::{convert, Options};
use std::io::{self, BufRead, Write};

let opts = Options::default();
let stdin = io::stdin();
let mut stdout = io::stdout().lock();

writeln!(stdout, "Paste SQL, end with EOF (Ctrl-D):")?;
let mut buf = String::new();
for line in stdin.lock().lines() {
    buf.push_str(&line?);
    buf.push('\n');
}

let (commands, warnings) = convert(&buf, &opts)?;
for w in &warnings {
    eprintln!("warn: {}", w.message);
}
print!("{commands}");
# Ok::<_, Box<dyn std::error::Error>>(())
```

## Generated rustdoc

For the canonical, always-current API reference:

```sh
cargo doc --open --no-deps -p loco-generate-via-sql
```

Includes runnable doctests for each public function.
