//! SQL type → Loco type-name mapping.

use crate::ir::SqlTypeRepr;

/// Outcome of a type lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocoTypeResult {
    pub loco_type: &'static str,
    /// Some(reason) when we fell back to `string` for an unknown SQL type.
    pub fallback_warning: Option<String>,
}

/// Map a SQL type representation to a Loco scaffold type name.
pub(crate) fn loco_type(repr: &SqlTypeRepr) -> LocoTypeResult {
    if repr.array {
        return LocoTypeResult { loco_type: "array", fallback_warning: None };
    }

    let canon = repr.canonical.to_uppercase();

    let base = match canon.as_str() {
        "UUID" => "uuid",

        "BOOL" | "BOOLEAN" => "bool",
        "TINYINT" if repr.args.first().copied() == Some(1) => "bool",
        "TINYINT" | "SMALLINT" | "INT2" | "MEDIUMINT" => "small_int",
        "INT" | "INT4" | "INTEGER" | "SERIAL" => "int",
        "BIGINT" | "INT8" | "BIGSERIAL" => "big_int",

        "REAL" | "FLOAT4" => "float",
        "DOUBLE" | "DOUBLE PRECISION" | "FLOAT8" | "FLOAT" => "double",
        "NUMERIC" | "DECIMAL" => "decimal",
        "MONEY" => "money",

        "CHAR" | "CHARACTER" | "VARCHAR" | "NVARCHAR" | "CHARACTER VARYING" => "string",
        "TEXT" | "MEDIUMTEXT" | "LONGTEXT" | "CLOB" => "text",

        "DATE" => "date",
        "TIME" => "string",
        "TIMESTAMP" | "DATETIME" => "date_time",
        "TIMESTAMPTZ" | "TIMESTAMP WITH TIME ZONE" => "tstz",

        "JSON" => "json",
        "JSONB" => "jsonb",
        "ARRAY" => "array",

        "BYTEA" | "BLOB" | "MEDIUMBLOB" | "LONGBLOB" => "blob",
        "BINARY" => "binary_len",
        "VARBINARY" => "var_binary",

        _ => {
            return LocoTypeResult {
                loco_type: "string",
                fallback_warning: Some(format!(
                    "unknown SQL type '{}', mapped to string", repr.canonical
                )),
            };
        }
    };

    // Apply unsigned modifier to integer types.
    let final_type = if repr.unsigned {
        match base {
            "small_int" => "small_unsigned",
            "int" => "unsigned",
            "big_int" => "big_unsigned",
            other => other,
        }
    } else {
        base
    };

    LocoTypeResult { loco_type: final_type, fallback_warning: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(canonical: &str) -> SqlTypeRepr {
        SqlTypeRepr {
            canonical: canonical.to_string(),
            args: vec![],
            unsigned: false,
            array: false,
        }
    }
    fn r_args(canonical: &str, args: Vec<u64>) -> SqlTypeRepr {
        let mut x = r(canonical);
        x.args = args;
        x
    }
    fn r_unsigned(canonical: &str) -> SqlTypeRepr {
        let mut x = r(canonical);
        x.unsigned = true;
        x
    }
    fn r_array(canonical: &str) -> SqlTypeRepr {
        let mut x = r(canonical);
        x.array = true;
        x
    }

    fn assert_maps_to(repr: SqlTypeRepr, expected: &str) {
        let result = loco_type(&repr);
        assert_eq!(result.loco_type, expected, "for {:?}", repr);
        assert!(result.fallback_warning.is_none(), "unexpected warning for {:?}", repr);
    }

    #[test] fn uuid()              { assert_maps_to(r("UUID"), "uuid"); }
    #[test] fn bool_named()        { assert_maps_to(r("BOOL"), "bool"); }
    #[test] fn boolean_named()     { assert_maps_to(r("BOOLEAN"), "bool"); }
    #[test] fn tinyint_one_bool()  { assert_maps_to(r_args("TINYINT", vec![1]), "bool"); }
    #[test] fn tinyint_plain()     { assert_maps_to(r("TINYINT"), "small_int"); }
    #[test] fn smallint()          { assert_maps_to(r("SMALLINT"), "small_int"); }
    #[test] fn int2()              { assert_maps_to(r("INT2"), "small_int"); }
    #[test] fn mediumint()         { assert_maps_to(r("MEDIUMINT"), "small_int"); }
    #[test] fn int_named()         { assert_maps_to(r("INT"), "int"); }
    #[test] fn int4()              { assert_maps_to(r("INT4"), "int"); }
    #[test] fn integer()           { assert_maps_to(r("INTEGER"), "int"); }
    #[test] fn serial()            { assert_maps_to(r("SERIAL"), "int"); }
    #[test] fn bigint()            { assert_maps_to(r("BIGINT"), "big_int"); }
    #[test] fn int8()              { assert_maps_to(r("INT8"), "big_int"); }
    #[test] fn bigserial()         { assert_maps_to(r("BIGSERIAL"), "big_int"); }
    #[test] fn smallint_unsigned() { assert_maps_to(r_unsigned("SMALLINT"), "small_unsigned"); }
    #[test] fn int_unsigned()      { assert_maps_to(r_unsigned("INT"), "unsigned"); }
    #[test] fn bigint_unsigned()   { assert_maps_to(r_unsigned("BIGINT"), "big_unsigned"); }
    #[test] fn real()              { assert_maps_to(r("REAL"), "float"); }
    #[test] fn float4()            { assert_maps_to(r("FLOAT4"), "float"); }
    #[test] fn double()            { assert_maps_to(r("DOUBLE"), "double"); }
    #[test] fn double_precision()  { assert_maps_to(r("DOUBLE PRECISION"), "double"); }
    #[test] fn float8()            { assert_maps_to(r("FLOAT8"), "double"); }
    #[test] fn float_no_prec()     { assert_maps_to(r("FLOAT"), "double"); }
    #[test] fn numeric()           { assert_maps_to(r_args("NUMERIC", vec![10, 2]), "decimal"); }
    #[test] fn decimal_t()         { assert_maps_to(r("DECIMAL"), "decimal"); }
    #[test] fn money()             { assert_maps_to(r("MONEY"), "money"); }
    #[test] fn char_t()            { assert_maps_to(r("CHAR"), "string"); }
    #[test] fn character()         { assert_maps_to(r("CHARACTER"), "string"); }
    #[test] fn varchar()           { assert_maps_to(r_args("VARCHAR", vec![255]), "string"); }
    #[test] fn nvarchar()          { assert_maps_to(r("NVARCHAR"), "string"); }
    #[test] fn character_varying() { assert_maps_to(r("CHARACTER VARYING"), "string"); }
    #[test] fn text_t()            { assert_maps_to(r("TEXT"), "text"); }
    #[test] fn mediumtext()        { assert_maps_to(r("MEDIUMTEXT"), "text"); }
    #[test] fn longtext()          { assert_maps_to(r("LONGTEXT"), "text"); }
    #[test] fn clob()              { assert_maps_to(r("CLOB"), "text"); }
    #[test] fn date_t()            { assert_maps_to(r("DATE"), "date"); }
    #[test] fn time_falls_back()   { assert_maps_to(r("TIME"), "string"); }
    #[test] fn timestamp_t()       { assert_maps_to(r("TIMESTAMP"), "date_time"); }
    #[test] fn datetime_t()        { assert_maps_to(r("DATETIME"), "date_time"); }
    #[test] fn timestamptz()       { assert_maps_to(r("TIMESTAMPTZ"), "tstz"); }
    #[test] fn timestamp_with_tz() { assert_maps_to(r("TIMESTAMP WITH TIME ZONE"), "tstz"); }
    #[test] fn json_t()            { assert_maps_to(r("JSON"), "json"); }
    #[test] fn jsonb_t()           { assert_maps_to(r("JSONB"), "jsonb"); }
    #[test] fn array_named()       { assert_maps_to(r("ARRAY"), "array"); }
    #[test] fn array_postfix()     { assert_maps_to(r_array("INT"), "array"); }
    #[test] fn bytea()             { assert_maps_to(r("BYTEA"), "blob"); }
    #[test] fn blob_named()        { assert_maps_to(r("BLOB"), "blob"); }
    #[test] fn mediumblob()        { assert_maps_to(r("MEDIUMBLOB"), "blob"); }
    #[test] fn longblob()          { assert_maps_to(r("LONGBLOB"), "blob"); }
    #[test] fn binary_n()          { assert_maps_to(r_args("BINARY", vec![16]), "binary_len"); }
    #[test] fn varbinary_n()       { assert_maps_to(r_args("VARBINARY", vec![16]), "var_binary"); }

    #[test]
    fn unknown_falls_back_to_string_with_warning() {
        let result = loco_type(&r("WIDGET"));
        assert_eq!(result.loco_type, "string");
        assert!(result.fallback_warning.is_some());
        assert!(result.fallback_warning.unwrap().contains("WIDGET"));
    }
}
