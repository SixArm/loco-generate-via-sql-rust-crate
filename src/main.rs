use std::io::{Read, Write};
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use loco_generate_via_sql::{
    ConvertError, Dialect, Options, ScaffoldKind, convert,
};

/// Convert SQL `CREATE TABLE` statements from stdin into
/// `cargo loco generate scaffold` commands on stdout.
#[derive(Debug, Parser)]
#[command(
    name = "loco-generate-via-sql",
    version,
    about,
    long_about = LONG_ABOUT,
    after_help = AFTER_HELP,
)]
struct Args {
    /// SQL dialect to parse the input with.
    #[arg(
        short,
        long,
        value_enum,
        default_value_t = DialectArg::Postgres,
        long_help = "\
SQL dialect for parsing the input.

  postgres  PostgreSQL — recognizes SERIAL, UUID, JSONB, BYTEA, TIMESTAMPTZ,
            schema-qualified names, [] arrays.
  mysql     MySQL — recognizes UNSIGNED modifier, TINYINT(1) booleans,
            MEDIUMINT, MEDIUMBLOB/LONGBLOB, backtick-quoted identifiers.
  sqlite    SQLite — recognizes INTEGER PRIMARY KEY AUTOINCREMENT and
            integer affinity.
  generic   Permissive fallback — accepts most syntax from any dialect.

When in doubt, try `generic` first. Switch to a specific dialect only if
something mis-parses."
    )]
    dialect: DialectArg,

    /// Scaffold template flag appended to each command.
    #[arg(
        short,
        long,
        value_enum,
        default_value_t = KindArg::Htmx,
        long_help = "\
Scaffold template flag appended to each `cargo loco generate scaffold`
command.

  htmx  --htmx (HTMX views — Loco's recommended dynamic UI scaffold)
  html  --html (server-rendered HTML views)
  api   --api  (JSON API only, no views)
  none  (no flag — useful when piping into another tool)"
    )]
    kind: KindArg,
}

const LONG_ABOUT: &str = "\
loco-generate-via-sql reads SQL from stdin and writes one
`cargo loco generate scaffold` command to stdout per CREATE TABLE.

The output is plain shell commands — pipe into `bash` to run, redirect to
a file to commit a build script, or pipe through other tools to massage
before running.

Behavior:
  - id, created_at, updated_at columns are skipped (Loco generates them).
  - NOT NULL → `!`, single-column UNIQUE → `^`. Both → `!^`.
  - REFERENCES tbl(col) → `<singular>:references[:custom_col]` (3 rules).
  - Multi-column UNIQUE constraints are silently dropped.
  - GENERATED ALWAYS AS … columns are skipped with a stderr warning.
  - Unknown SQL types fall back to `string` with a stderr warning.
  - Tables are emitted in source order (no topological sort).

Exit codes:
  0  success (warnings on stderr don't affect this)
  1  SQL parse error
  2  I/O error (stdin read or stdout write failed)";

const AFTER_HELP: &str = "\
EXAMPLES:
  # Print scaffold commands to terminal:
  loco-generate-via-sql < schema.sql

  # Save to a runnable script:
  loco-generate-via-sql < schema.sql > setup.sh

  # Run scaffolds inline (-e aborts on any failure):
  loco-generate-via-sql < schema.sql | sh -e

  # MySQL schema, API-only scaffolds:
  loco-generate-via-sql -d mysql -k api < shop.sql

  # Suppress warnings:
  loco-generate-via-sql < schema.sql 2>/dev/null

DOCS:
  Full guide:        https://github.com/sixarm/loco-generate-via-sql
  Type mapping:      docs/types.md
  Foreign-key rules: docs/foreign-keys.md
  Dialect notes:     docs/dialects.md
  Troubleshooting:   docs/troubleshooting.md
  Tutorial:          docs/tutorial.md";

#[derive(Copy, Clone, Debug, ValueEnum)]
enum DialectArg {
    Postgres,
    Mysql,
    Sqlite,
    Generic,
}

impl From<DialectArg> for Dialect {
    fn from(d: DialectArg) -> Self {
        match d {
            DialectArg::Postgres => Dialect::Postgres,
            DialectArg::Mysql => Dialect::MySql,
            DialectArg::Sqlite => Dialect::SQLite,
            DialectArg::Generic => Dialect::Generic,
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum KindArg {
    Htmx,
    Html,
    Api,
    None,
}

impl From<KindArg> for ScaffoldKind {
    fn from(k: KindArg) -> Self {
        match k {
            KindArg::Htmx => ScaffoldKind::Htmx,
            KindArg::Html => ScaffoldKind::Html,
            KindArg::Api => ScaffoldKind::Api,
            KindArg::None => ScaffoldKind::None,
        }
    }
}

fn main() -> ExitCode {
    let args = Args::parse();

    let mut sql = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut sql) {
        eprintln!("error: failed to read stdin: {}", e);
        return ExitCode::from(2);
    }

    let opts = Options {
        dialect: args.dialect.into(),
        kind: args.kind.into(),
    };

    match convert(&sql, &opts) {
        Ok((out, warnings)) => {
            for w in &warnings {
                eprintln!("warn: {}", w.message);
            }
            if let Err(e) = std::io::stdout().write_all(out.as_bytes()) {
                eprintln!("error: failed to write stdout: {}", e);
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Err(ConvertError::Parse(msg)) => {
            eprintln!("error: SQL parse failed: {}", msg);
            ExitCode::from(1)
        }
        Err(ConvertError::Io(e)) => {
            eprintln!("error: {}", e);
            ExitCode::from(2)
        }
    }
}
