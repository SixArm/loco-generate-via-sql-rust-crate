//! Dialect-neutral intermediate representation.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColumnDef {
    pub name: String,
    pub sql_type: SqlTypeRepr,
    pub not_null: bool,
    pub unique: bool,
    pub primary_key: bool,
    pub generated: bool,
    pub fk: Option<ForeignKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqlTypeRepr {
    /// Upper-case base type, e.g. "VARCHAR", "TINYINT", "TIMESTAMPTZ".
    pub canonical: String,
    /// Optional length/precision args, in order. e.g. `VARCHAR(255)` → `[255]`.
    pub args: Vec<u64>,
    /// MySQL `UNSIGNED` modifier.
    pub unsigned: bool,
    /// True for `int[]` or `ARRAY` types.
    pub array: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForeignKey {
    pub target_table: String,
    pub target_column: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_a_simple_column() {
        let col = ColumnDef {
            name: "title".to_string(),
            sql_type: SqlTypeRepr {
                canonical: "VARCHAR".to_string(),
                args: vec![255],
                unsigned: false,
                array: false,
            },
            not_null: true,
            unique: false,
            primary_key: false,
            generated: false,
            fk: None,
        };
        assert_eq!(col.name, "title");
        assert_eq!(col.sql_type.canonical, "VARCHAR");
        assert!(col.not_null);
    }

    #[test]
    fn build_a_table() {
        let t = TableDef {
            name: "posts".to_string(),
            columns: vec![],
        };
        assert_eq!(t.name, "posts");
        assert!(t.columns.is_empty());
    }
}
