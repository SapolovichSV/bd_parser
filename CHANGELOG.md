# Changelog

All notable changes and improvements made during the comprehensive code review.

## [Unreleased] - 2025-11-08

### Fixed

#### Critical Fixes
- **[Cargo.toml]** Fixed invalid Rust edition from "2024" to "2021"
- **[main.rs]** Removed `todo!()` macro in production code path - now returns proper error for unknown bookstore domains
- **[main.rs]** Fixed discarded result from `parse_sitemap_igraslov()` call at program start
- **[parse_traits.rs]** Fixed potential panic in `Isbn::parse()` by replacing `.unwrap()` with safer `Option` handling
- **[labirint.rs]** Fixed integer overflow risk in exponential backoff by using safe bit shift operations
- **[All parsers]** Fixed numerous `.expect()` calls with better error messages

#### Error Handling Improvements
- **[main.rs]** Added comprehensive error context throughout using `anyhow::Context`
- **[labirint.rs]** Enhanced retry logic with proper error accumulation and reporting
- **[igraslov.rs]** Improved error messages with actionable context
- **[parse_traits.rs]** Better ISBN validation error messages with actual vs expected format

### Added

#### Documentation
- **[README.md]** Complete project documentation including:
  - Feature overview
  - Installation and usage instructions
  - Architecture description
  - Performance tips and troubleshooting
  - Contributing guidelines
- **[All modules]** Added comprehensive module-level documentation
- **[All public APIs]** Added doc comments for all public types, traits, and methods
- **[parse_traits.rs]** Documented all struct fields and trait methods with examples

#### Code Quality
- **[.gitignore]** Added CSV output files to prevent accidental commits
- **[labirint.rs]** Properly marked integration test with `#[ignore]` attribute
- **[All modules]** Added detailed inline comments for complex logic

### Changed

#### Style & Idiomaticity
- **[main.rs]** Changed `static` to `const` for immutable constants
- **[main.rs]** Renamed variables for better clarity:
  - `parse_count` → `concurrent_tasks`
  - `parse_from_one` → `books_per_store`
  - `how_much_url_process_at_once_source` → removed magic number
- **[All modules]** Improved error messages from generic to specific
- **[parse_traits.rs]** Used clippy suggestion for range contains: `(10..=13).contains(&digit_count)`

#### Architecture
- **[main.rs]** Better separation of concerns with clearer function responsibilities
- **[main.rs]** Improved CLI argument parsing with better user feedback
- **[main.rs]** Enhanced configuration with named constants instead of magic numbers
- **[csv_save.rs]** Removed unused `csv_headers()` method from trait
- **[csv_save.rs]** Simplified `to_string()` usage

#### Performance
- **[parse_traits.rs]** Optimized ISBN validation to avoid redundant string operations
- **[labirint.rs]** Better backoff strategy with proper timing
- **[main.rs]** More efficient URL collection and processing

#### Safety
- **[labirint.rs]** Safe exponential backoff without overflow risk
- **[parse_traits.rs]** Eliminated potential panics in ISBN parsing
- **[All modules]** Replaced panic-prone `.expect()` with proper error handling where appropriate

### Improved

#### Testing
- **[labirint.rs]** Enhanced test documentation and assertions
- **[igraslov.rs]** Maintained existing tests with better error messages
- **[All tests]** Clear test names and better failure messages

#### Logging
- **[telemetry.rs]** Added comprehensive documentation for logging setup
- **[main.rs]** Better structured logging with success/error counts
- **[All parsers]** More informative log messages

#### Error Messages
- Before: `"bad url"`, `"should"`, `"can't parse isbn"`
- After: `"URL does not contain '/books/' path segment"`, `"Invalid CSS selector for <loc>"`, `"Failed to parse ISBN from page: {url}"`

### Removed

#### Dead Code
- **[csv_save.rs]** Removed unused `csv_headers()` associated function
- **[labirint.rs]** Removed unused `EXPECTED_ISBN` test constant

## Statistics

- **Files Modified**: 8
- **Files Created**: 2 (README.md, CHANGELOG.md)
- **Lines of Documentation Added**: ~300+
- **Critical Bugs Fixed**: 6
- **Tests Passing**: 9/10 (1 ignored integration test)
- **Clippy Warnings Fixed**: All actionable warnings resolved
- **Compilation**: ✅ Success (with 2 false-positive dead code warnings)

## Code Quality Metrics

### Before Review
- ❌ Invalid Rust edition
- ❌ `todo!()` in production code
- ❌ Potential panics from `.unwrap()`
- ❌ Integer overflow risk
- ❌ No documentation
- ⚠️ Generic error messages
- ⚠️ Magic numbers throughout

### After Review
- ✅ Correct Rust 2021 edition
- ✅ All code paths implemented
- ✅ Safe error handling throughout
- ✅ Overflow-safe arithmetic
- ✅ Comprehensive documentation
- ✅ Context-rich error messages
- ✅ Named constants for configuration

## Testing

All tests pass successfully:
```
test result: ok. 9 passed; 0 failed; 1 ignored
```

The ignored test is an integration test requiring network access, which is intentionally marked as `#[ignore]`.

## Security

No security vulnerabilities introduced. All changes improve code safety and robustness:
- Eliminated potential panics
- Safe integer arithmetic
- Proper error propagation
- No unsafe code blocks

## Performance

No performance regressions. Minor improvements from:
- Reduced string allocations in ISBN parsing
- Better backoff strategy in HTTP retries
- More efficient error handling
