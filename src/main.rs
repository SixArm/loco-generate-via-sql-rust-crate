use std::io::{Read, Write};
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use loco_generate_scaffold_via_sql_schema::{
    ConvertError, Dialect, Options, ScaffoldKind, convert,
};

/// Convert SQL CREATE TABLE statements from stdin into
/// `cargo loco generate scaffold` commands on stdout.
#[derive(Debug, Parser)]
#[command(name = "loco-generate-scaffold-via-sql-schema", version, about)]
struct Args {
    /// SQL dialect to parse with.
    #[arg(short, long, value_enum, default_value_t = DialectArg::Postgres)]
    dialect: DialectArg,

    /// Scaffold template kind to append to each command.
    #[arg(short, long, value_enum, default_value_t = KindArg::Htmx)]
    kind: KindArg,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum DialectArg { Postgres, Mysql, Sqlite, Generic }

impl From<DialectArg> for Dialect {
    fn from(d: DialectArg) -> Self {
        match d {
            DialectArg::Postgres => Dialect::Postgres,
            DialectArg::Mysql    => Dialect::MySql,
            DialectArg::Sqlite   => Dialect::SQLite,
            DialectArg::Generic  => Dialect::Generic,
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum KindArg { Htmx, Html, Api, None }

impl From<KindArg> for ScaffoldKind {
    fn from(k: KindArg) -> Self {
        match k {
            KindArg::Htmx => ScaffoldKind::Htmx,
            KindArg::Html => ScaffoldKind::Html,
            KindArg::Api  => ScaffoldKind::Api,
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

    let opts = Options { dialect: args.dialect.into(), kind: args.kind.into() };

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
