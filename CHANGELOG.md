# Changelog
All changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.5] - 2024-04-02
### Changed
- Improved crate documentation and synced `README.md` with `lib.rs` using `cargo-readme`.

## [0.2.4] - 2024-03-27
### Changed
- Enabled documentation to be generated for all features.

## [0.2.3] - 2024-03-07
### Fixed
- Corrected import location of `PgMetadataLookup`.

## [0.2.2] - 2024-01-05
### Added
- Support for `R2D2Connection` through an optional feature flag, `r2d2`.
- Support for `MultiConnection` through the `MultiConnectionHelper` trait for each
  connection type provided by the crate.

## [0.2.1] - 2023-12-19
### Added
- Support for `MigrationConnection`

### Changed
- Allowed the recording of statements through an optional feature flag, `statement-fields`.

### Fixed
- Incorrect `db.system` string on sqlite `LoadConnection`.

## [0.2.0] - 2023-06-22
### Added
- Support for `UpdateAndFetchResults`.

### Changed
- Target diesel version to `>=2.1`.

## [0.1.6] - 2022-06-13
### Changed
- Implemented UpdateAndFetchResults trait to support save DSL.

## [0.1.5] - 2021-06-18
### Changed
- Updated ipnetwork bounds to match diesel.

## [0.1.4] - 2021-03-16
### Changed
- Updated ipnetwork bounds to match diesel.
- Corrected various clippy warnings.

## [0.1.3] - 2020-08-17
####
- Added a status badge to manifest and README.

### Changed
- Switched to canonical OpenTelemetry field names for connection information. 

## [0.1.2] - 2020-08-12
### Changed
- Fixed broken feature flags.

## [0.1.1] - 2020-08-12
### Added
- Instrumented PostgreSQL connections now collect the connection configuration.
- Added feature flags for other connection types.

## [0.1.0] - 2020-08-12
### Added
- Initial implementation.

[0.1.0]: https://crates.io/crates/diesel-tracing/0.1.0
[0.1.1]: https://crates.io/crates/diesel-tracing/0.1.1
[0.1.2]: https://crates.io/crates/diesel-tracing/0.1.2
[0.1.3]: https://crates.io/crates/diesel-tracing/0.1.3
[0.1.4]: https://crates.io/crates/diesel-tracing/0.1.4
[0.1.5]: https://crates.io/crates/diesel-tracing/0.1.5
[0.1.6]: https://crates.io/crates/diesel-tracing/0.1.6
[0.2.0]: https://crates.io/crates/diesel-tracing/0.2.0
[0.2.1]: https://crates.io/crates/diesel-tracing/0.2.1
[0.2.2]: https://crates.io/crates/diesel-tracing/0.2.2
[0.2.3]: https://crates.io/crates/diesel-tracing/0.2.3
[0.2.4]: https://crates.io/crates/diesel-tracing/0.2.4
[0.2.5]: https://crates.io/crates/diesel-tracing/0.2.5
