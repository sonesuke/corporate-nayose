//! Verification harness for the real `data/` tables. Ignored by default (the
//! raw CSVs are gitignored); run via `mise verify-tables` after `mise tables`.
//!
//! Expected numbers were measured against the raw files on 2026-09:
//! Kihonjoho_UTF-8.csv and Jigyousyojoho_UTF-8.csv.

use std::fs;
use std::path::PathBuf;

use corporate_nayose::tables::{companies, establishments};
use polars::prelude::*;

fn table_dir() -> PathBuf {
    PathBuf::from("data/parquet")
}

fn scan(name: &str) -> DataFrame {
    let path = table_dir().join(name);
    LazyFrame::scan_parquet(
        PlRefPath::try_from_path(&path).unwrap(),
        ScanArgsParquet::default(),
    )
    .unwrap()
    .collect()
    .unwrap()
}

/// Distinct non-null values in a String column.
fn distinct(df: &DataFrame, name: &str) -> usize {
    let series = df.column(name).unwrap().as_materialized_series();
    let has_null = series.null_count() > 0;
    series.unique().unwrap().len() - usize::from(has_null)
}

fn count_where(df: &DataFrame, name: &str, f: impl Fn(Option<&str>) -> bool) -> usize {
    let ca = df.column(name).unwrap().str().unwrap();
    (0..df.height()).filter(|i| f(ca.get(*i))).count()
}

#[test]
#[ignore]
fn companies_table_matches_raw_data() {
    let df = scan(companies::OUTPUT);

    // Raw CSV: 5,832,807 records, every one with a distinct corporate number.
    assert_eq!(df.height(), 5_832_807);
    assert_eq!(distinct(&df, "corporate_number"), 5_832_807);

    // status: null (現存) 5,050,997 + 閉鎖 781,810
    let status = df.column("status").unwrap().str().unwrap();
    let active = (0..df.height())
        .filter(|i| status.get(*i).is_none())
        .count();
    let closed = count_where(&df, "status", |s| s == Some("閉鎖"));
    assert_eq!(active, 5_050_997);
    assert_eq!(closed, 781_810);

    // Spot check: 国立国会図書館 (1000011000005)
    let number = df.column("corporate_number").unwrap().str().unwrap();
    let row = (0..df.height()).find(|i| number.get(*i) == Some("1000011000005"));
    assert!(row.is_some(), "国立国会図書館 row missing");
    let row = row.unwrap();
    let s = |name: &str| df.column(name).unwrap().str().unwrap().get(row);
    assert_eq!(s("corporate_name"), Some("国立国会図書館"));
    assert_eq!(s("corporate_name_kana"), Some("コクリツコッカイトショカン"));
    assert_eq!(s("corporate_name_en"), Some("National Diet Library"));
    assert_eq!(s("postal_code"), Some("1000014"));
    assert_eq!(s("prefecture_code"), Some("13"));
    assert_eq!(s("municipality_code"), Some("101"));
    assert_eq!(s("organization_type"), Some("101"));
    assert_eq!(s("status"), None);
}

#[test]
#[ignore]
fn establishments_table_matches_raw_data() {
    let df = scan(establishments::OUTPUT);

    // Raw CSV: 2,886,775 records across 2,854,199 corporate numbers.
    assert_eq!(df.height(), 2_886_775);
    assert_eq!(distinct(&df, "corporate_number"), 2_854_199);

    // Spot check: 宮城県後期高齢者医療広域連合 (1000020049727), 被保険者数 8
    let number = df.column("corporate_number").unwrap().str().unwrap();
    let insured = df.column("insured_employees").unwrap().u32().unwrap();
    let rows: Vec<usize> = (0..df.height())
        .filter(|i| number.get(*i) == Some("1000020049727"))
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(insured.get(rows[0]), Some(8));
}

#[test]
#[ignore]
fn tables_exist() {
    for name in [companies::OUTPUT, establishments::OUTPUT] {
        let path = table_dir().join(name);
        assert!(fs::metadata(&path).is_ok(), "missing {path:?}");
    }
}
