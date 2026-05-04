//! IR → `cargo loco generate scaffold` command lines.

use crate::Options;
use crate::ScaffoldKind;
use crate::Warning;
use crate::ir::{ColumnDef, ForeignKey, TableDef};
use crate::types::loco_type;

const SKIP_COLUMNS: &[&str] = &["id", "created_at", "updated_at"];

pub(crate) fn emit_command(table: &TableDef, opts: &Options, warnings: &mut Vec<Warning>) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push("cargo loco generate scaffold".to_string());
    parts.push(table.name.clone());

    for col in &table.columns {
        if let Some(field) = emit_field(col, warnings, &table.name) {
            parts.push(field);
        }
    }

    match opts.kind {
        ScaffoldKind::Htmx => parts.push("--htmx".to_string()),
        ScaffoldKind::Html => parts.push("--html".to_string()),
        ScaffoldKind::Api  => parts.push("--api".to_string()),
        ScaffoldKind::None => {}
    }

    parts.join(" ")
}

/// Render the per-column field spec, or None if the column is skipped.
pub(crate) fn emit_field(col: &ColumnDef, warnings: &mut Vec<Warning>, table_name: &str) -> Option<String> {
    if SKIP_COLUMNS.contains(&col.name.as_str()) {
        return None;
    }
    if col.generated {
        warnings.push(Warning {
            message: format!(
                "skipping generated column '{}.{}' (Loco scaffold has no equivalent)",
                table_name, col.name,
            ),
        });
        return None;
    }

    if let Some(fk) = &col.fk {
        return Some(emit_fk_field(&col.name, fk));
    }

    let mapped = loco_type(&col.sql_type);
    if let Some(msg) = mapped.fallback_warning {
        warnings.push(Warning {
            message: format!("{} (column '{}.{}')", msg, table_name, col.name),
        });
    }

    let mut suffix = String::new();
    if col.not_null || col.primary_key {
        suffix.push('!');
    }
    if col.unique {
        suffix.push('^');
    }

    Some(format!("{}:{}{}", col.name, mapped.loco_type, suffix))
}

/// Implements the three FK rules from the spec.
fn emit_fk_field(col_name: &str, fk: &ForeignKey) -> String {
    let target_singular = depluralize(&fk.target_table);

    if let Some(prefix) = col_name.strip_suffix("_id") {
        let target_match = depluralize(&fk.target_table);
        if prefix == target_match {
            return format!("{}:references", prefix);
        }
        // Rule 2: ends in _id but prefix doesn't match.
        return format!("{}:references:{}", target_singular, col_name);
    }

    // Rule 3: column does not end in _id.
    format!("{}:references:{}", target_singular, col_name)
}

fn depluralize(s: &str) -> String {
    if let Some(stem) = s.strip_suffix('s') {
        stem.to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::SqlTypeRepr;

    fn col(name: &str, canonical: &str) -> ColumnDef {
        ColumnDef {
            name: name.to_string(),
            sql_type: SqlTypeRepr {
                canonical: canonical.to_string(),
                args: vec![],
                unsigned: false,
                array: false,
            },
            not_null: false,
            unique: false,
            primary_key: false,
            generated: false,
            fk: None,
        }
    }

    fn run(c: ColumnDef) -> (Option<String>, Vec<Warning>) {
        let mut w = Vec::new();
        let f = emit_field(&c, &mut w, "t");
        (f, w)
    }

    #[test]
    fn skips_id() {
        let (f, _) = run(col("id", "INT"));
        assert!(f.is_none());
    }

    #[test]
    fn skips_created_at() {
        let (f, _) = run(col("created_at", "TIMESTAMPTZ"));
        assert!(f.is_none());
    }

    #[test]
    fn skips_updated_at() {
        let (f, _) = run(col("updated_at", "TIMESTAMPTZ"));
        assert!(f.is_none());
    }

    #[test]
    fn skips_generated_columns_with_warning() {
        let mut c = col("computed", "INT");
        c.generated = true;
        let (f, w) = run(c);
        assert!(f.is_none());
        assert_eq!(w.len(), 1);
        assert!(w[0].message.contains("computed"));
    }

    #[test]
    fn plain_string_no_suffix() {
        let (f, _) = run(col("title", "TEXT"));
        assert_eq!(f.as_deref(), Some("title:text"));
    }

    #[test]
    fn not_null_adds_bang() {
        let mut c = col("title", "VARCHAR");
        c.not_null = true;
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("title:string!"));
    }

    #[test]
    fn unique_adds_caret() {
        let mut c = col("email", "TEXT");
        c.unique = true;
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("email:text^"));
    }

    #[test]
    fn not_null_unique_appends_both() {
        let mut c = col("email", "TEXT");
        c.not_null = true;
        c.unique = true;
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("email:text!^"));
    }

    #[test]
    fn fk_rule_1_matching_prefix_uses_bare_references() {
        let mut c = col("author_id", "INT");
        c.fk = Some(ForeignKey { target_table: "authors".into(), target_column: "id".into() });
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("author:references"));
    }

    #[test]
    fn fk_rule_1_matches_singular_target_table() {
        let mut c = col("author_id", "INT");
        c.fk = Some(ForeignKey { target_table: "author".into(), target_column: "id".into() });
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("author:references"));
    }

    #[test]
    fn fk_rule_2_mismatched_prefix_uses_custom_col() {
        let mut c = col("owner_id", "INT");
        c.fk = Some(ForeignKey { target_table: "users".into(), target_column: "id".into() });
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("user:references:owner_id"));
    }

    #[test]
    fn fk_rule_3_no_id_suffix_uses_custom_col() {
        let mut c = col("author", "INT");
        c.fk = Some(ForeignKey { target_table: "users".into(), target_column: "id".into() });
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("user:references:author"));
    }

    #[test]
    fn fk_strips_not_null_and_unique_suffixes() {
        let mut c = col("author_id", "INT");
        c.not_null = true;
        c.unique = true;
        c.fk = Some(ForeignKey { target_table: "authors".into(), target_column: "id".into() });
        let (f, _) = run(c);
        assert_eq!(f.as_deref(), Some("author:references"));
    }

    #[test]
    fn unknown_type_falls_back_to_string_with_warning() {
        let (f, w) = run(col("x", "WIDGET"));
        assert_eq!(f.as_deref(), Some("x:string"));
        assert_eq!(w.len(), 1);
        assert!(w[0].message.contains("WIDGET"));
    }

    fn make_table() -> TableDef {
        let mut id = col("id", "INT");
        id.primary_key = true;
        let mut title = col("title", "VARCHAR");
        title.not_null = true;
        let mut email = col("email", "TEXT");
        email.unique = true;
        TableDef {
            name: "posts".into(),
            columns: vec![id, title, email],
        }
    }

    #[test]
    fn emit_command_with_htmx_kind() {
        let t = make_table();
        let opts = Options { dialect: crate::Dialect::Postgres, kind: ScaffoldKind::Htmx };
        let mut w = Vec::new();
        let cmd = emit_command(&t, &opts, &mut w);
        assert_eq!(
            cmd,
            "cargo loco generate scaffold posts title:string! email:text^ --htmx"
        );
        assert!(w.is_empty());
    }

    #[test]
    fn emit_command_with_html_kind() {
        let t = make_table();
        let opts = Options { dialect: crate::Dialect::Postgres, kind: ScaffoldKind::Html };
        let mut w = Vec::new();
        let cmd = emit_command(&t, &opts, &mut w);
        assert!(cmd.ends_with(" --html"));
    }

    #[test]
    fn emit_command_with_api_kind() {
        let t = make_table();
        let opts = Options { dialect: crate::Dialect::Postgres, kind: ScaffoldKind::Api };
        let mut w = Vec::new();
        let cmd = emit_command(&t, &opts, &mut w);
        assert!(cmd.ends_with(" --api"));
    }

    #[test]
    fn emit_command_with_kind_none_omits_flag() {
        let t = make_table();
        let opts = Options { dialect: crate::Dialect::Postgres, kind: ScaffoldKind::None };
        let mut w = Vec::new();
        let cmd = emit_command(&t, &opts, &mut w);
        assert_eq!(
            cmd,
            "cargo loco generate scaffold posts title:string! email:text^"
        );
    }

    #[test]
    fn emit_command_table_with_no_user_columns() {
        let t = TableDef {
            name: "tags".into(),
            columns: vec![{
                let mut id = col("id", "INT");
                id.primary_key = true;
                id
            }],
        };
        let opts = Options { dialect: crate::Dialect::Postgres, kind: ScaffoldKind::Htmx };
        let mut w = Vec::new();
        let cmd = emit_command(&t, &opts, &mut w);
        assert_eq!(cmd, "cargo loco generate scaffold tags --htmx");
    }
}
