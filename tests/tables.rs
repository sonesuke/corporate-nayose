//! Fixture round-trip tests for the table pipeline: raw CSV (BOM, CRLF,
//! quoted commas/newlines, leading zeros) → parquet.

use std::fs;
use std::path::{Path, PathBuf};

use corporate_nayose::tables::{TableStats, companies, establishments};
use polars::prelude::*;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("corporate-nayose-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_fixture(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, contents.as_bytes()).unwrap();
    path
}

/// Dates are compared via ISO strings (Date → String cast round-trips
/// `YYYY-MM-DD`), which avoids a direct chrono dependency in tests.
fn date_as_string(df: &DataFrame, name: &str, i: usize) -> Option<String> {
    df.column(name)
        .unwrap()
        .cast(&DataType::String)
        .unwrap()
        .str()
        .unwrap()
        .get(i)
        .map(str::to_string)
}

fn scan_parquet(path: &Path) -> DataFrame {
    LazyFrame::scan_parquet(
        PlRefPath::try_from_path(path).unwrap(),
        ScanArgsParquet::default(),
    )
    .unwrap()
    .collect()
    .unwrap()
}

#[test]
fn companies_fixture_roundtrip() {
    let dir = temp_dir("companies");
    let input = write_fixture(
        &dir,
        companies::SOURCE,
        include_str!("fixtures/companies.csv"),
    );
    let output = dir.join(companies::OUTPUT);

    let stats: TableStats = companies::build(&input, &output).unwrap();

    // 6 rows (a–f) plus the embedded blank line; (c) duplicates (a); (f) has
    // no corporate number.
    assert!(stats.input_rows >= 6, "input_rows: {:?}", stats.input_rows);
    assert!(stats.null_key_rows >= 1);
    assert_eq!(stats.duplicate_rows, 1);
    assert_eq!(stats.output_rows, 4);

    let df = scan_parquet(&output);
    assert_eq!(df.height(), 4);

    // dtypes
    assert_eq!(
        df.column("corporate_number").unwrap().dtype(),
        &DataType::String
    );
    assert_eq!(df.column("capital").unwrap().dtype(), &DataType::UInt64);
    assert_eq!(df.column("employees").unwrap().dtype(), &DataType::UInt32);
    assert_eq!(
        df.column("founded_year").unwrap().dtype(),
        &DataType::UInt32
    );
    assert_eq!(df.column("closed_date").unwrap().dtype(), &DataType::Date);
    // provenance metadata columns must be gone
    assert!(df.column("出典元").is_err());

    let number = df.column("corporate_number").unwrap().str().unwrap();
    let name = df.column("corporate_name").unwrap().str().unwrap();
    let status = df.column("status").unwrap().str().unwrap();
    let postal = df.column("postal_code").unwrap().str().unwrap();

    // keep-first dedup: row (a) wins over (c) — updated_date stays 2018
    assert_eq!(
        date_as_string(&df, "updated_date", 0).as_deref(),
        Some("2018-04-02")
    );

    // row (a): active — status null
    assert_eq!(status.get(0), None);
    assert_eq!(postal.get(0), Some("1000014"));

    // row (b): closed
    assert_eq!(status.get(1), Some("閉鎖"));
    assert_eq!(
        date_as_string(&df, "closed_date", 1).as_deref(),
        Some("2017-03-31")
    );

    // row (d): numerics + leading-zero postal code preserved as String
    assert_eq!(postal.get(2), Some("0310075"));
    assert_eq!(
        df.column("capital").unwrap().u64().unwrap().get(2),
        Some(10_000_000)
    );
    assert_eq!(
        df.column("employees").unwrap().u32().unwrap().get(2),
        Some(42)
    );
    assert_eq!(
        df.column("founded_year").unwrap().u32().unwrap().get(2),
        Some(1990)
    );
    assert_eq!(
        date_as_string(&df, "incorporation_date", 2).as_deref(),
        Some("1990-04-02")
    );

    // row (e): quoted comma and embedded newline survive
    assert_eq!(name.get(3), Some("株式会社テスト,商社"));
    assert_eq!(
        df.column("business_summary").unwrap().str().unwrap().get(3),
        Some("概要1行目\n概要2行目")
    );
    assert_eq!(number.get(3), Some("4000040000001"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn establishments_fixture_roundtrip() {
    let dir = temp_dir("establishments");
    let input = write_fixture(
        &dir,
        establishments::SOURCE,
        include_str!("fixtures/establishments.csv"),
    );
    let output = dir.join(establishments::OUTPUT);

    let stats = establishments::build(&input, &output).unwrap();

    // No dedup: one legal entity may have many establishments.
    assert_eq!(stats.null_key_rows, 0);
    assert_eq!(stats.duplicate_rows, 0);
    assert_eq!(stats.output_rows, 3);

    let df = scan_parquet(&output);
    assert_eq!(df.height(), 3);

    let number = df.column("corporate_number").unwrap().str().unwrap();
    // 本社 and 支店 share the same corporate number and both survive
    assert_eq!(number.get(0), Some("1000020049727"));
    assert_eq!(number.get(1), Some("1000020049727"));

    let insured = df.column("insured_employees").unwrap().u32().unwrap();
    assert_eq!(insured.get(0), Some(8));
    assert_eq!(insured.get(1), None); // empty field → null
    assert_eq!(insured.get(2), Some(14));

    assert_eq!(
        date_as_string(&df, "coverage_end_date", 2).as_deref(),
        Some("2024-05-01")
    );
    assert_eq!(date_as_string(&df, "coverage_end_date", 0), None);

    let _ = fs::remove_dir_all(&dir);
}
