# Changelog
All changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.5]: https://crates.io/crates/diesel-tracing/0.1.5
[0.1.4]: https://crates.io/crates/diesel-tracing/0.1.4
[0.1.3]: https://crates.io/crates/diesel-tracing/0.1.3
[0.1.2]: https://crates.io/crates/diesel-tracing/0.1.2
[0.1.1]: https://crates.io/crates/diesel-tracing/0.1.1
[0.1.0]: https://crates.io/crates/diesel-tracing/0.1.0
