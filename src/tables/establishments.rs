//! 事業所テーブル: Jigyousyojoho_UTF-8.csv → establishments.parquet

use std::path::Path;

use polars::prelude::*;

use super::{
    ColType, ColumnMapping, TableStats, count_input, count_output, project, scan_csv_strings,
    sink_parquet,
};

pub const SOURCE: &str = "Jigyousyojoho_UTF-8.csv";
pub const OUTPUT: &str = "establishments.parquet";
pub const KEY: &str = "法人番号";

/// 15列のうち名寄せに必要な9列のみ。source 列名は半角括弧(基本情報とは逆なので注意)。
/// 事業所のユニークIDは元データに存在しないため dedup しない(1法人に複数事業所は正常)。
const COLUMNS: &ColumnMapping = &[
    (KEY, "corporate_number", ColType::Text),
    ("商号または名称", "corporate_name", ColType::Text),
    ("登記住所", "registered_address", ColType::Text),
    ("名称", "establishment_name", ColType::Text),
    ("名称(カナ)", "establishment_name_kana", ColType::Text),
    ("事業所住所", "establishment_address", ColType::Text),
    (
        "事業所住所(カナ)",
        "establishment_address_kana",
        ColType::Text,
    ),
    ("被保険者数", "insured_employees", ColType::U32),
    ("全喪年月日", "coverage_end_date", ColType::Date),
];

pub fn build(input: &Path, output: &Path) -> PolarsResult<TableStats> {
    let (input_rows, valid_keys) = count_input(input, KEY)?;
    let table = project(scan_csv_strings(input)?, COLUMNS)?.filter(
        col("corporate_number")
            .is_not_null()
            .and(col("corporate_number").neq(lit(""))),
    );
    sink_parquet(table, output)?;
    let output_rows = count_output(output)?;
    let stats = TableStats {
        input_rows,
        null_key_rows: input_rows - valid_keys,
        duplicate_rows: 0,
        output_rows,
    };
    stats.log("establishments");
    Ok(stats)
}
