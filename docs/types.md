# Type mapping reference

Mapping is dialect-aware: `sqlparser` parses dialect-specific keywords, and
this tool normalizes the result to one of Loco's canonical type names.
Length and precision arguments are parsed (and visible in the IR) but are
not used for type selection — Loco's type names don't carry them.

## Full mapping

| SQL type                                         | Loco type     | Notes |
|--------------------------------------------------|---------------|-------|
| `UUID`                                           | `uuid`        | |
| `BOOL`, `BOOLEAN`                                | `bool`        | |
| `TINYINT(1)`                                     | `bool`        | MySQL convention. |
| `TINYINT`                                        | `small_int`   | Without `(1)` — see below. |
| `SMALLINT`, `INT2`, `MEDIUMINT`                  | `small_int`   | `MEDIUMINT` is 24-bit in MySQL but maps here for closeness — see [Edge cases](#edge-cases). |
| `INT`, `INT4`, `INTEGER`, `SERIAL`               | `int`         | Postgres `SERIAL` is auto-increment; the auto-increment is implicit in Loco. |
| `BIGINT`, `INT8`, `BIGSERIAL`                    | `big_int`     | |
| `SMALLINT UNSIGNED`                              | `small_unsigned` | MySQL only. |
| `INT UNSIGNED`                                   | `unsigned`    | MySQL only. |
| `BIGINT UNSIGNED`                                | `big_unsigned`| MySQL only. |
| `REAL`, `FLOAT4`                                 | `float`       | 32-bit. |
| `DOUBLE`, `DOUBLE PRECISION`, `FLOAT8`           | `double`      | 64-bit. |
| `FLOAT` (no precision)                           | `double`      | See [Edge cases](#edge-cases). |
| `NUMERIC`, `NUMERIC(p,s)`                        | `decimal`     | Precision/scale parsed but discarded. |
| `DECIMAL`, `DECIMAL(p,s)`                        | `decimal`     | |
| `MONEY`                                          | `money`       | Postgres. |
| `CHAR`, `CHARACTER`                              | `string`      | |
| `VARCHAR`, `VARCHAR(n)`                          | `string`      | Length parsed but discarded. |
| `NVARCHAR`, `CHARACTER VARYING`                  | `string`      | |
| `TEXT`, `MEDIUMTEXT`, `LONGTEXT`, `CLOB`         | `text`        | |
| `DATE`                                           | `date`        | |
| `TIME`                                           | `string`      | Loco has no `time` type — falls back. |
| `TIMESTAMP`, `DATETIME`                          | `date_time`   | |
| `TIMESTAMPTZ`, `TIMESTAMP WITH TIME ZONE`        | `tstz`        | Postgres. |
| `JSON`                                           | `json`        | |
| `JSONB`                                          | `jsonb`       | Postgres. |
| `<type>[]`, `ARRAY`                              | `array`       | Postgres array; element type is currently dropped. |
| `BYTEA`                                          | `blob`        | Postgres. |
| `BLOB`, `MEDIUMBLOB`, `LONGBLOB`                 | `blob`        | MySQL. |
| `BINARY(n)`                                      | `binary_len`  | Length carried as IR arg but Loco's `binary_len` is fixed. |
| `VARBINARY(n)`                                   | `var_binary`  | |
| anything else                                    | `string`      | Emits a stderr warning. |

## Suffixes

After the type name, one or both of these suffixes may appear (in order,
no separator):

- **`!`** — column is `NOT NULL` or has inline `PRIMARY KEY`.
- **`^`** — column has inline `UNIQUE`, or appears as the only column in a
  table-level `UNIQUE (col)` constraint.

A `NOT NULL UNIQUE` column emits `name:type!^`.

Foreign-key columns (those carrying a `REFERENCES` clause) **never** get
suffixes — Loco's `references` template owns nullability and uniqueness for
those fields. See [foreign-keys.md](foreign-keys.md).

## Unsigned promotion

For MySQL columns marked `UNSIGNED`, the integer type is promoted:

| Base | Becomes |
|------|---------|
| `small_int` | `small_unsigned` |
| `int`       | `unsigned` |
| `big_int`   | `big_unsigned` |
| anything else | unchanged (`UNSIGNED REAL` etc. stay `float`) |

The promotion only applies to the three integer types Loco offers an unsigned
counterpart for. `TINYINT(1) UNSIGNED` is still `bool` (the `(1)` rule wins
before the unsigned promotion).

## Edge cases

### `MEDIUMINT` truncation

MySQL `MEDIUMINT` is 24-bit (range ~±8 million). Loco has no `medium_int`,
so this tool maps it to `small_int` (16-bit). Values outside `±32767`
won't fit. If you have `MEDIUMINT` columns with real-world values above
the SMALLINT range, consider editing the output to `int` (32-bit) by
hand, or change your schema to `INT` to avoid the surprise.

### `FLOAT` precision rules

Per the SQL standard, `FLOAT(p)` with `p ≤ 24` is single-precision and
`FLOAT(p)` with `p ≥ 25` is double-precision. This tool maps any `FLOAT`
to `double` regardless of precision. If you specifically need 32-bit, use
`REAL` or `FLOAT4`.

### `BINARY(n)` vs `VARBINARY(n)`

Loco's `binary_len` is a fixed-length binary type; `var_binary` is variable.
The length argument is parsed and held in the IR but isn't currently used
to select a different Loco type. If your `BINARY(16)` column needs to be
exactly 16 bytes, edit the migration after scaffolding.

### Postgres arrays

`int[]` and `varchar[]` both map to `array` (the element type is dropped).
Loco's array support is broad enough that the element type isn't carried
in the scaffold field syntax.

### Unknown types

If `sqlparser` parses a type but this tool doesn't have an explicit rule
for it (e.g., dialect-specific extensions), the column is emitted as
`<col>:string` with a warning:

```text
warn: unknown SQL type 'WIDGET', mapped to string (column 't.q')
```

This is a deliberate "don't crash on stray dialects" fallback. Edit the
output if `string` is wrong.

## Source of truth

Loco's canonical type names are documented at
[loco.rs/docs/the-app/models/](https://loco.rs/docs/the-app/models/). This
tool tracks the names current as of the spec's `2026-05-04` revision.
Older Loco versions used different names (`ts` instead of `tstz`, `bigint`
instead of `big_int`, etc.) — the design spec records the migration.
