//! SQL → IR via `sqlparser`.

use crate::ConvertError;
use crate::Dialect;
use crate::ir::{ColumnDef, ForeignKey, SqlTypeRepr, TableDef};

use sqlparser::ast::{
    DataType, Expr, GeneratedAs, ObjectName, ObjectNamePart, Statement, TableConstraint,
};
use sqlparser::dialect::{
    Dialect as SqlDialect, GenericDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect,
};
use sqlparser::parser::Parser;

/// Parse SQL text and return one TableDef per CREATE TABLE statement,
/// in source order. Non-CREATE-TABLE statements are skipped silently.
pub(crate) fn parse_sql(sql: &str, dialect: Dialect) -> Result<Vec<TableDef>, ConvertError> {
    let d: Box<dyn SqlDialect> = match dialect {
        Dialect::Postgres => Box::new(PostgreSqlDialect {}),
        Dialect::MySql => Box::new(MySqlDialect {}),
        Dialect::SQLite => Box::new(SQLiteDialect {}),
        Dialect::Generic => Box::new(GenericDialect {}),
    };
    let stmts = Parser::parse_sql(&*d, sql).map_err(|e| ConvertError::Parse(e.to_string()))?;

    let mut out = Vec::new();
    for stmt in stmts {
        if let Statement::CreateTable(ct) = stmt {
            out.push(table_from_create(&ct.name, &ct.columns, &ct.constraints));
        }
    }
    Ok(out)
}

/// Extract the plain string value from an ObjectName part.
fn ident_from_part(part: &ObjectNamePart) -> Option<&str> {
    match part {
        ObjectNamePart::Identifier(ident) => Some(&ident.value),
        _ => None,
    }
}

/// Use the last segment of a possibly-qualified name (`public.users` → `users`).
fn object_name_last(name: &ObjectName) -> String {
    name.0
        .last()
        .and_then(|p| ident_from_part(p))
        .map(|s| s.to_string())
        .unwrap_or_default()
}

fn table_from_create(
    name: &ObjectName,
    columns: &[sqlparser::ast::ColumnDef],
    constraints: &[TableConstraint],
) -> TableDef {
    let table_name = object_name_last(name);

    // Pre-scan constraints for single-column UNIQUE and FOREIGN KEY.
    let mut unique_singles: Vec<String> = Vec::new();
    let mut tbl_fks: Vec<(String, ForeignKey)> = Vec::new();

    for c in constraints {
        match c {
            TableConstraint::Unique(uc) if uc.columns.len() == 1 => {
                // IndexColumn wraps OrderByExpr wraps Expr::Identifier(Ident)
                if let Expr::Identifier(ident) = &uc.columns[0].column.expr {
                    unique_singles.push(ident.value.clone());
                }
            }
            TableConstraint::ForeignKey(fk)
                if fk.columns.len() == 1 && fk.referred_columns.len() == 1 =>
            {
                let target_table = object_name_last(&fk.foreign_table);
                let target_column = fk.referred_columns[0].value.clone();
                tbl_fks.push((
                    fk.columns[0].value.clone(),
                    ForeignKey { target_table, target_column },
                ));
            }
            _ => {}
        }
    }

    let mut cols = Vec::with_capacity(columns.len());
    for c in columns {
        let mut col = column_from_sqlparser(c);
        if unique_singles.iter().any(|n| n == &col.name) {
            col.unique = true;
        }
        if col.fk.is_none()
            && let Some((_, fk)) = tbl_fks.iter().find(|(n, _)| n == &col.name)
        {
            col.fk = Some(fk.clone());
        }
        cols.push(col);
    }

    TableDef { name: table_name, columns: cols }
}

fn column_from_sqlparser(c: &sqlparser::ast::ColumnDef) -> ColumnDef {
    use sqlparser::ast::ColumnOption;

    let mut not_null = false;
    let mut unique = false;
    let mut primary_key = false;
    let mut generated = false;
    let mut fk: Option<ForeignKey> = None;

    for opt in &c.options {
        match &opt.option {
            ColumnOption::NotNull => not_null = true,
            ColumnOption::PrimaryKey(_) => {
                primary_key = true;
                not_null = true;
            }
            ColumnOption::Unique(_) => {
                unique = true;
            }
            ColumnOption::ForeignKey(fk_constraint) => {
                let target_table = object_name_last(&fk_constraint.foreign_table);
                let target_column = fk_constraint
                    .referred_columns
                    .first()
                    .map(|i| i.value.clone())
                    .unwrap_or_else(|| "id".to_string());
                fk = Some(ForeignKey { target_table, target_column });
            }
            ColumnOption::Generated { generated_as, .. } => {
                match generated_as {
                    GeneratedAs::Always | GeneratedAs::ByDefault | GeneratedAs::ExpStored => {
                        generated = true;
                    }
                }
            }
            _ => {}
        }
    }

    let sql_type = sql_type_repr(&c.data_type);
    ColumnDef {
        name: c.name.value.clone(),
        sql_type,
        not_null,
        unique,
        primary_key,
        generated,
        fk,
    }
}

fn char_length_to_u64(cl: &sqlparser::ast::CharacterLength) -> Option<u64> {
    use sqlparser::ast::CharacterLength;
    match cl {
        CharacterLength::IntegerLength { length, .. } => Some(*length),
        CharacterLength::Max => None,
    }
}

fn sql_type_repr(dt: &DataType) -> SqlTypeRepr {
    use DataType::*;
    let (canonical, args, unsigned, array) = match dt {
        Uuid => ("UUID".to_string(), vec![], false, false),
        Boolean | Bool => ("BOOL".to_string(), vec![], false, false),
        TinyInt(n) => ("TINYINT".to_string(), n.iter().copied().collect(), false, false),
        TinyIntUnsigned(n) => ("TINYINT".to_string(), n.iter().copied().collect(), true, false),
        SmallInt(n) => ("SMALLINT".to_string(), n.iter().copied().collect(), false, false),
        SmallIntUnsigned(n) => ("SMALLINT".to_string(), n.iter().copied().collect(), true, false),
        MediumInt(n) => ("MEDIUMINT".to_string(), n.iter().copied().collect(), false, false),
        MediumIntUnsigned(n) => ("MEDIUMINT".to_string(), n.iter().copied().collect(), true, false),
        Int(n) | Integer(n) => ("INT".to_string(), n.iter().copied().collect(), false, false),
        IntUnsigned(n) | IntegerUnsigned(n) => ("INT".to_string(), n.iter().copied().collect(), true, false),
        BigInt(n) => ("BIGINT".to_string(), n.iter().copied().collect(), false, false),
        BigIntUnsigned(n) => ("BIGINT".to_string(), n.iter().copied().collect(), true, false),
        Real => ("REAL".to_string(), vec![], false, false),
        Double(_) | DoublePrecision => ("DOUBLE".to_string(), vec![], false, false),
        Float(_) => ("FLOAT".to_string(), vec![], false, false),
        Numeric(info) => ("NUMERIC".to_string(), exact_args(info), false, false),
        Decimal(info) => ("DECIMAL".to_string(), exact_args(info), false, false),
        Char(n) | Character(n) => (
            "CHAR".to_string(),
            n.iter().filter_map(char_length_to_u64).collect(),
            false,
            false,
        ),
        Varchar(n) | CharVarying(n) | CharacterVarying(n) => (
            "VARCHAR".to_string(),
            n.iter().filter_map(char_length_to_u64).collect(),
            false,
            false,
        ),
        Nvarchar(n) => (
            "NVARCHAR".to_string(),
            n.iter().filter_map(char_length_to_u64).collect(),
            false,
            false,
        ),
        Text => ("TEXT".to_string(), vec![], false, false),
        MediumText => ("MEDIUMTEXT".to_string(), vec![], false, false),
        LongText => ("LONGTEXT".to_string(), vec![], false, false),
        Clob(n) => ("CLOB".to_string(), n.iter().copied().collect(), false, false),
        Date => ("DATE".to_string(), vec![], false, false),
        Time(_, _) => ("TIME".to_string(), vec![], false, false),
        Datetime(_) => ("DATETIME".to_string(), vec![], false, false),
        Timestamp(_, tz) => {
            use sqlparser::ast::TimezoneInfo;
            match tz {
                TimezoneInfo::Tz | TimezoneInfo::WithTimeZone => {
                    ("TIMESTAMPTZ".to_string(), vec![], false, false)
                }
                _ => ("TIMESTAMP".to_string(), vec![], false, false),
            }
        }
        JSON => ("JSON".to_string(), vec![], false, false),
        JSONB => ("JSONB".to_string(), vec![], false, false),
        Bytea => ("BYTEA".to_string(), vec![], false, false),
        Blob(n) => ("BLOB".to_string(), n.iter().copied().collect(), false, false),
        Binary(n) => ("BINARY".to_string(), n.iter().copied().collect(), false, false),
        Varbinary(_) => ("VARBINARY".to_string(), vec![], false, false),
        Array(_) => ("ARRAY".to_string(), vec![], false, true),
        // Catch-all: stringify whatever sqlparser saw.
        other => (format!("{}", other).to_uppercase(), vec![], false, false),
    };

    SqlTypeRepr { canonical, args, unsigned, array }
}

fn exact_args(info: &sqlparser::ast::ExactNumberInfo) -> Vec<u64> {
    use sqlparser::ast::ExactNumberInfo;
    match info {
        ExactNumberInfo::None => vec![],
        ExactNumberInfo::Precision(p) => vec![*p],
        ExactNumberInfo::PrecisionAndScale(p, s) => vec![*p, *s as u64],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_pg(sql: &str) -> Vec<TableDef> {
        parse_sql(sql, Dialect::Postgres).expect("parse")
    }

    #[test]
    fn parses_a_simple_table() {
        let tables = parse_pg("CREATE TABLE posts (id SERIAL PRIMARY KEY, title TEXT NOT NULL);");
        assert_eq!(tables.len(), 1);
        let t = &tables[0];
        assert_eq!(t.name, "posts");
        assert_eq!(t.columns.len(), 2);
        assert_eq!(t.columns[0].name, "id");
        assert!(t.columns[0].primary_key);
        assert_eq!(t.columns[1].name, "title");
        assert_eq!(t.columns[1].sql_type.canonical, "TEXT");
        assert!(t.columns[1].not_null);
    }

    #[test]
    fn preserves_source_order_across_multiple_tables() {
        let sql = "CREATE TABLE a (id SERIAL PRIMARY KEY); \
                   CREATE TABLE b (id SERIAL PRIMARY KEY); \
                   CREATE TABLE c (id SERIAL PRIMARY KEY);";
        let tables = parse_pg(sql);
        let names: Vec<_> = tables.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn handles_if_not_exists() {
        let tables = parse_pg("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY);");
        assert_eq!(tables[0].name, "users");
    }

    #[test]
    fn skips_non_create_table_statements() {
        let sql = "CREATE INDEX idx ON posts(title); \
                   CREATE TABLE posts (id SERIAL PRIMARY KEY); \
                   ALTER TABLE posts ADD COLUMN x INT;";
        let tables = parse_pg(sql);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name, "posts");
    }

    #[test]
    fn detects_inline_unique() {
        let tables = parse_pg("CREATE TABLE u (email TEXT UNIQUE);");
        assert!(tables[0].columns[0].unique);
    }

    #[test]
    fn detects_table_level_single_column_unique() {
        let tables = parse_pg("CREATE TABLE u (email TEXT, UNIQUE (email));");
        assert!(tables[0].columns[0].unique);
    }

    #[test]
    fn ignores_table_level_multi_column_unique() {
        let tables = parse_pg("CREATE TABLE u (a TEXT, b TEXT, UNIQUE (a, b));");
        assert!(!tables[0].columns[0].unique);
        assert!(!tables[0].columns[1].unique);
    }

    #[test]
    fn detects_inline_references() {
        let sql = "CREATE TABLE comments (post_id INT REFERENCES posts(id));";
        let tables = parse_pg(sql);
        let fk = tables[0].columns[0].fk.as_ref().expect("fk");
        assert_eq!(fk.target_table, "posts");
        assert_eq!(fk.target_column, "id");
    }

    #[test]
    fn detects_table_level_foreign_key() {
        let sql = "CREATE TABLE comments (post_id INT, FOREIGN KEY (post_id) REFERENCES posts(id));";
        let tables = parse_pg(sql);
        let fk = tables[0].columns[0].fk.as_ref().expect("fk");
        assert_eq!(fk.target_table, "posts");
    }

    #[test]
    fn unwraps_quoted_identifiers() {
        let sql = r#"CREATE TABLE "User" ("Email" TEXT);"#;
        let tables = parse_pg(sql);
        assert_eq!(tables[0].name, "User");
        assert_eq!(tables[0].columns[0].name, "Email");
    }

    #[test]
    fn strips_schema_qualifier_from_table_name() {
        let tables = parse_pg("CREATE TABLE public.users (id SERIAL PRIMARY KEY);");
        assert_eq!(tables[0].name, "users");
    }

    #[test]
    fn captures_varchar_length() {
        let tables = parse_pg("CREATE TABLE p (title VARCHAR(255));");
        let t = &tables[0].columns[0].sql_type;
        assert_eq!(t.canonical, "VARCHAR");
        assert_eq!(t.args, vec![255]);
    }

    #[test]
    fn detects_generated_columns() {
        let sql = "CREATE TABLE p (a INT, b INT GENERATED ALWAYS AS (a + 1) STORED);";
        let tables = parse_pg(sql);
        assert!(tables[0].columns[1].generated);
    }

    #[test]
    fn mysql_unsigned_modifier() {
        let tables = parse_sql(
            "CREATE TABLE p (n INT UNSIGNED);",
            Dialect::MySql,
        ).expect("parse");
        assert!(tables[0].columns[0].sql_type.unsigned);
    }

    #[test]
    fn parse_error_is_reported() {
        let result = parse_sql("CREATE TABLE (bad", Dialect::Postgres);
        assert!(result.is_err());
    }
}
