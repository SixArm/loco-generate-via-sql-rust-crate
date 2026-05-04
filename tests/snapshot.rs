use loco_generate_via_sql::{Dialect, Options, ScaffoldKind, convert};

fn opts(d: Dialect, k: ScaffoldKind) -> Options {
    Options { dialect: d, kind: k }
}

#[test]
fn simple_postgres_table() {
    let sql = "\
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    body  TEXT NOT NULL,
    slug  TEXT UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
";
    let (out, w) = convert(sql, &opts(Dialect::Postgres, ScaffoldKind::Htmx)).unwrap();
    assert_eq!(
        out,
        "cargo loco generate scaffold posts title:string! body:text! slug:text^ --htmx\n"
    );
    assert!(w.is_empty());
}

#[test]
fn fk_chain_preserves_source_order() {
    let sql = "\
CREATE TABLE authors (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    author_id INT NOT NULL REFERENCES authors(id),
    title TEXT NOT NULL
);
CREATE TABLE comments (
    id SERIAL PRIMARY KEY,
    post_id INT NOT NULL REFERENCES posts(id),
    owner_id INT NOT NULL REFERENCES authors(id),
    body TEXT NOT NULL
);
";
    let (out, _) = convert(sql, &opts(Dialect::Postgres, ScaffoldKind::Htmx)).unwrap();
    let expected = "\
cargo loco generate scaffold authors name:text! --htmx

cargo loco generate scaffold posts author:references title:text! --htmx

cargo loco generate scaffold comments post:references author:references:owner_id body:text! --htmx
";
    assert_eq!(out, expected);
}

#[test]
fn mysql_unsigned_and_tinyint_one() {
    let sql = "\
CREATE TABLE widgets (
    id INT UNSIGNED NOT NULL,
    qty INT UNSIGNED NOT NULL,
    big BIGINT UNSIGNED NOT NULL,
    active TINYINT(1) NOT NULL
);
";
    let (out, _) = convert(sql, &opts(Dialect::MySql, ScaffoldKind::Htmx)).unwrap();
    assert_eq!(
        out,
        "cargo loco generate scaffold widgets qty:unsigned! big:big_unsigned! active:bool! --htmx\n"
    );
}

#[test]
fn sqlite_autoincrement() {
    let sql = "\
CREATE TABLE notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    body  TEXT
);
";
    let (out, _) = convert(sql, &opts(Dialect::SQLite, ScaffoldKind::Htmx)).unwrap();
    assert_eq!(
        out,
        "cargo loco generate scaffold notes title:text! body:text --htmx\n"
    );
}

#[test]
fn multi_column_unique_is_dropped_silently() {
    let sql = "\
CREATE TABLE memberships (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    group_id INT NOT NULL,
    UNIQUE (user_id, group_id)
);
";
    let (out, w) = convert(sql, &opts(Dialect::Postgres, ScaffoldKind::Htmx)).unwrap();
    // Neither user_id nor group_id should carry `^`.
    assert_eq!(
        out,
        "cargo loco generate scaffold memberships user_id:int! group_id:int! --htmx\n"
    );
    assert!(w.is_empty());
}

#[test]
fn generated_column_is_skipped_with_warning() {
    let sql = "\
CREATE TABLE p (
    id SERIAL PRIMARY KEY,
    a INT NOT NULL,
    b INT GENERATED ALWAYS AS (a + 1) STORED
);
";
    let (out, w) = convert(sql, &opts(Dialect::Postgres, ScaffoldKind::Htmx)).unwrap();
    assert_eq!(out, "cargo loco generate scaffold p a:int! --htmx\n");
    assert_eq!(w.len(), 1);
    assert!(w[0].message.contains("p.b"));
}

#[test]
fn schema_qualified_table_name_uses_last_segment() {
    let sql = "CREATE TABLE public.users (id SERIAL PRIMARY KEY, email TEXT NOT NULL UNIQUE);";
    let (out, _) = convert(sql, &opts(Dialect::Postgres, ScaffoldKind::Htmx)).unwrap();
    assert_eq!(out, "cargo loco generate scaffold users email:text!^ --htmx\n");
}

#[test]
fn binary_smoke_test_via_assert_cmd() {
    use assert_cmd::Command;
    let mut cmd = Command::cargo_bin("loco-generate-via-sql").unwrap();
    cmd.write_stdin("CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);")
        .assert()
        .success()
        .stdout("cargo loco generate scaffold posts title:text! --htmx\n");
}

#[test]
fn binary_smoke_test_kind_none() {
    use assert_cmd::Command;
    let mut cmd = Command::cargo_bin("loco-generate-via-sql").unwrap();
    cmd.args(["--kind", "none"])
        .write_stdin("CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);")
        .assert()
        .success()
        .stdout("cargo loco generate scaffold posts title:text!\n");
}

#[test]
fn binary_exits_nonzero_on_parse_error() {
    use assert_cmd::Command;
    let mut cmd = Command::cargo_bin("loco-generate-via-sql").unwrap();
    cmd.write_stdin("CREATE TABLE (oops")
        .assert()
        .failure()
        .code(1);
}
