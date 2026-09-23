//! 企業テーブル: Kihonjoho_UTF-8.csv → companies.parquet

use std::path::Path;

use polars::prelude::*;

use super::{
    ColType, ColumnMapping, TableStats, count_input, count_output, project, scan_csv_strings,
    sink_parquet,
};

pub const SOURCE: &str = "Kihonjoho_UTF-8.csv";
pub const OUTPUT: &str = "companies.parquet";
pub const KEY: &str = "法人番号";

/// 96列のうち名寄せに必要な26列のみ。残りはデータ品質-*/出典元-*/データ取込頻度-* などの
/// 来歴メタデータ列。source 列名は全角括弧に注意(例: 商号または名称(カナ))。
const COLUMNS: &ColumnMapping = &[
    (KEY, "corporate_number", ColType::Text),
    ("商号または名称", "corporate_name", ColType::Text),
    (
        "商号または名称（カナ）",
        "corporate_name_kana",
        ColType::Text,
    ),
    ("商号または名称（英字）", "corporate_name_en", ColType::Text),
    ("登記記録の閉鎖等年月日", "closed_date", ColType::Date),
    ("登記記録の閉鎖等の事由", "closed_reason", ColType::Text),
    ("登記住所", "registered_address", ColType::Text),
    ("郵便番号", "postal_code", ColType::Text),
    ("都道府県", "prefecture", ColType::Text),
    ("都道府県コード", "prefecture_code", ColType::Text),
    ("市区町村（郡）", "municipality", ColType::Text),
    ("市区町村コード", "municipality_code", ColType::Text),
    ("番地以下", "address_detail", ColType::Text),
    ("組織種別", "organization_type", ColType::Text),
    ("処理区分", "process_category", ColType::Text),
    ("訂正区分", "correction_category", ColType::Text),
    ("状態", "status", ColType::Text),
    ("代表者名称", "representative_name", ColType::Text),
    ("資本金", "capital", ColType::U64),
    ("従業員数", "employees", ColType::U32),
    ("事業概要", "business_summary", ColType::Text),
    ("WebサイトURL", "website_url", ColType::Text),
    ("創業年", "founded_year", ColType::U32),
    ("事業種目", "business_type", ColType::Text),
    ("設立年月日", "incorporation_date", ColType::Date),
    ("更新年月日", "updated_date", ColType::Date),
];

pub fn build(input: &Path, output: &Path) -> PolarsResult<TableStats> {
    let (input_rows, valid_keys) = count_input(input, KEY)?;
    let table = project(scan_csv_strings(input)?, COLUMNS)?
        .filter(
            col("corporate_number")
                .is_not_null()
                .and(col("corporate_number").neq(lit(""))),
        )
        .unique_stable_generic(
            Some(vec![col("corporate_number")]),
            UniqueKeepStrategy::First,
        );
    sink_parquet(table, output)?;
    let output_rows = count_output(output)?;
    let stats = TableStats {
        input_rows,
        null_key_rows: input_rows - valid_keys,
        duplicate_rows: valid_keys - output_rows,
        output_rows,
    };
    stats.log("companies");
    Ok(stats)
}
