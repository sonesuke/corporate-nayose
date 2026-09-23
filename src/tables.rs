//! Shared plumbing for building parquet tables from the raw registry CSVs.

use std::path::Path;
use std::sync::Arc;

use polars::prelude::*;

pub mod companies;
pub mod establishments;

/// Per-table counts, logged after each build and returned for tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableStats {
    pub input_rows: u32,
    pub null_key_rows: u32,
    pub duplicate_rows: u32,
    pub output_rows: u32,
}

impl TableStats {
    pub fn log(&self, name: &str) {
        eprintln!(
            "{name}: {} rows read, {} without corporate number, {} duplicates, {} written",
            self.input_rows, self.null_key_rows, self.duplicate_rows, self.output_rows
        );
    }
}

/// Output column type, applied in `project`. Kept as a tiny enum because
/// `DataType` is not const-constructible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColType {
    Text,
    U32,
    U64,
    Date,
}

impl ColType {
    fn dtype(self) -> DataType {
        match self {
            ColType::Text => DataType::String,
            ColType::U32 => DataType::UInt32,
            ColType::U64 => DataType::UInt64,
            ColType::Date => DataType::Date,
        }
    }
}

/// (source CSV column, output column, output type)
pub(crate) type ColumnMapping = [(&'static str, &'static str, ColType)];

/// Lazy CSV scan; every field is read as String so leading zeros
/// (postal codes, municipality codes) survive. Dtypes are applied per column
/// in `project`.
pub(crate) fn scan_csv_strings(path: &Path) -> PolarsResult<LazyFrame> {
    LazyCsvReader::new(PlRefPath::try_from_path(path)?)
        .with_has_header(true)
        .with_infer_schema_length(Some(0)) // => all columns String
        .finish()
}

/// `select(src AS dst, cast)` — resolves source names tolerating a BOM prefix
/// on the first column, in case the CSV reader does not strip it.
pub(crate) fn project(mut lf: LazyFrame, mapping: &ColumnMapping) -> PolarsResult<LazyFrame> {
    let schema = lf.collect_schema()?;
    let exprs = mapping
        .iter()
        .map(|(src, dst, ty)| {
            let name = schema
                .iter()
                .map(|(name, _)| name)
                .find(|n| n.as_str().strip_prefix('\u{feff}').unwrap_or(n.as_str()) == *src)
                .ok_or_else(|| {
                    PolarsError::ColumnNotFound(format!("column `{src}` not found in input").into())
                })?
                .clone();
            let expr = col(name);
            let expr = match *ty {
                // Real CSVs quote every field, so empty values arrive as ""
                // (not null): normalize them to null.
                ColType::Text => when(expr.clone().eq(lit("")))
                    .then(lit(NULL))
                    .otherwise(expr),
                ColType::Date => expr.str().to_date(StrptimeOptions {
                    format: Some("%Y-%m-%d".into()),
                    strict: false, // unparseable/empty -> null
                    exact: true,
                    cache: true,
                }),
                // NonStrict cast: empty/unparseable fields become null.
                _ => expr.cast(ty.dtype()),
            };
            // Alias last: when/then/otherwise and casts can rename the column.
            Ok(expr.alias(*dst))
        })
        .collect::<PolarsResult<Vec<_>>>()?;
    Ok(lf.select(exprs))
}

/// Streaming parquet sink (zstd default). Bounded memory; never materializes
/// the frame.
pub(crate) fn sink_parquet(lf: LazyFrame, output: &Path) -> PolarsResult<()> {
    if let Some(dir) = output.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let sink = lf.sink(
        SinkDestination::File {
            target: SinkTarget::Path(PlRefPath::try_from_path(output)?),
        },
        FileWriteFormat::Parquet(Arc::new(ParquetWriteOptions::default())),
        UnifiedSinkArgs::default(),
    )?;
    let _query = sink.collect_with_engine(Engine::Streaming)?;
    Ok(())
}

/// Cheap first pass: parses only the key column (projection pushdown makes
/// this fast even on the 96-column companies CSV).
/// Returns (rows, rows with a non-null key).
pub(crate) fn count_input(input: &Path, key_src: &str) -> PolarsResult<(u32, u32)> {
    let mut lf = scan_csv_strings(input)?;
    let schema = lf.collect_schema()?;
    let key = schema
        .iter()
        .map(|(name, _)| name)
        .find(|n| n.as_str().strip_prefix('\u{feff}').unwrap_or(n.as_str()) == key_src)
        .ok_or_else(|| PolarsError::ColumnNotFound(format!("column `{key_src}` not found").into()))?
        .clone();
    let df = lf
        .select([
            len().alias("rows"),
            col(key.clone())
                .is_not_null()
                .and(col(key).neq(lit("")))
                .sum()
                .alias("valid"),
        ])
        .collect()?;
    Ok((scalar_u32(&df, "rows")?, scalar_u32(&df, "valid")?))
}

pub(crate) fn count_output(output: &Path) -> PolarsResult<u32> {
    let lf = LazyFrame::scan_parquet(
        PlRefPath::try_from_path(output)?,
        ScanArgsParquet::default(),
    )?;
    let df = lf.select([len().alias("rows")]).collect()?;
    scalar_u32(&df, "rows")
}

fn scalar_u32(df: &DataFrame, name: &str) -> PolarsResult<u32> {
    Ok(df.column(name)?.u32()?.get(0).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_from_columns(names: &[&str], values: &[Vec<Option<&str>>]) -> PolarsResult<LazyFrame> {
        let cols = names
            .iter()
            .zip(values)
            .map(|(name, vals)| {
                Column::new(
                    PlSmallStr::from_str(name),
                    vals.iter()
                        .map(|v| v.map(str::to_string))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        Ok(DataFrame::new_infer_height(cols)?.lazy())
    }

    #[test]
    fn project_resolves_bom_prefixed_names_and_casts() {
        let lf = frame_from_columns(
            &["\u{feff}法人番号", "資本金"],
            &[
                vec![Some("1000011000005"), None],
                vec![Some("12345"), Some("67890")],
            ],
        )
        .unwrap();
        let out = project(
            lf,
            &[
                ("法人番号", "corporate_number", ColType::Text),
                ("資本金", "capital", ColType::U64),
            ],
        )
        .unwrap()
        .collect()
        .unwrap();
        assert_eq!(out.height(), 2);
        let number = out.column("corporate_number").unwrap();
        assert_eq!(number.dtype(), &DataType::String);
        assert_eq!(number.str().unwrap().get(0), Some("1000011000005"));
        assert_eq!(number.str().unwrap().get(1), None);
        let capital = out.column("capital").unwrap();
        assert_eq!(capital.dtype(), &DataType::UInt64);
        assert_eq!(capital.u64().unwrap().get(1), Some(67890));
    }

    #[test]
    fn project_fails_on_unknown_column() {
        let lf = frame_from_columns(&["法人番号"], &[vec![Some("1")]]).unwrap();
        let err = project(lf, &[("存在しない列", "x", ColType::Text)])
            .err()
            .expect("should fail");
        assert!(matches!(err, PolarsError::ColumnNotFound(_)));
    }
}
