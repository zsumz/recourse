//! Focused command-line grammar tests kept outside production modules.

use std::{ffi::OsString, path::PathBuf};

use recourse::catalog::{CodeNumber, Reservation};

use super::arguments::{CatalogPaths, Command, OutputFormat, parse};

fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

#[test]
fn cargo_prefix_and_catalog_paths_parse_strictly() {
    let command = parse(args(&[
        "recourse",
        "check",
        "--lock",
        "diagnostics/catalog.lock",
        "--current",
        "diagnostics/catalog.json",
        "--format",
        "json",
    ]))
    .unwrap_or_else(|error| panic!("valid check command must parse: {error}"));
    assert_eq!(
        command,
        Command::Check(CatalogPaths {
            current: PathBuf::from("diagnostics/catalog.json"),
            lock: PathBuf::from("diagnostics/catalog.lock"),
            format: OutputFormat::Json,
        })
    );
}

#[test]
fn accept_requires_an_explicit_breaking_flag() {
    let command = parse(args(&[
        "accept",
        "--current",
        "catalog.json",
        "--acknowledge-breaking",
        "--lock",
        "catalog.lock",
    ]))
    .unwrap_or_else(|error| panic!("valid accept command must parse: {error}"));
    assert_eq!(
        command,
        Command::Accept {
            paths: CatalogPaths {
                current: PathBuf::from("catalog.json"),
                lock: PathBuf::from("catalog.lock"),
                format: OutputFormat::Human,
            },
            acknowledge_breaking: true,
        }
    );
}

#[test]
fn reserve_supports_exact_positive_numbers() {
    let command = parse(args(&[
        "reserve",
        "--number",
        "42",
        "--lock",
        "catalog.lock",
    ]))
    .unwrap_or_else(|error| panic!("valid reservation must parse: {error}"));
    assert_eq!(
        command,
        Command::Reserve {
            lock: PathBuf::from("catalog.lock"),
            reservation: Reservation::Exact(CodeNumber::new(42)),
            format: OutputFormat::Human,
        }
    );
}

#[test]
fn retire_requires_reason_and_accepts_an_optional_replacement() {
    let command = parse(args(&[
        "retire",
        "--lock",
        "catalog.lock",
        "DSP-1004",
        "--reason",
        "Unified with dispatch failure.",
        "--replacement",
        "DSP-1009",
        "--format",
        "json",
    ]))
    .unwrap_or_else(|error| panic!("valid retirement must parse: {error}"));
    let Command::Retire {
        code,
        reason,
        replacement,
        format,
        ..
    } = command
    else {
        panic!("retire input must produce retire intent");
    };
    assert_eq!(code.to_string(), "DSP-1004");
    assert_eq!(reason, "Unified with dispatch failure.");
    assert_eq!(
        replacement.map(|code| code.to_string()).as_deref(),
        Some("DSP-1009")
    );
    assert_eq!(format, OutputFormat::Json);
    assert!(parse(args(&["retire", "--lock", "catalog.lock", "DSP-1004"])).is_err());
}

#[test]
fn missing_duplicate_and_unknown_options_fail_closed() {
    for values in [
        args(&["check", "--current", "catalog.json"]),
        args(&[
            "check",
            "--current",
            "one",
            "--current",
            "two",
            "--lock",
            "lock",
        ]),
        args(&["reserve", "--lock", "lock", "--surprise"]),
        args(&["reserve", "--lock", "lock", "--number", "0"]),
    ] {
        assert!(parse(values).is_err());
    }
}

#[test]
fn documentation_and_explanation_have_explicit_inputs() {
    let docs = parse(args(&[
        "docs",
        "--out",
        "docs/problems",
        "--lock",
        "catalog.lock",
        "--current",
        "catalog.json",
    ]));
    assert!(matches!(docs, Ok(Command::Docs { .. })));

    let explain = parse(args(&[
        "explain",
        "--current",
        "catalog.json",
        "DSP-1004",
        "--format",
        "json",
    ]));
    assert!(matches!(
        explain,
        Ok(Command::Explain {
            format: OutputFormat::Json,
            ..
        })
    ));
}
