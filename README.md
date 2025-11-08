# Book Parser (bd_parser)

A high-performance, concurrent web scraper for extracting book metadata from Russian bookstore websites.

## Features

- 🚀 **Concurrent Processing**: Configurable parallel parsing with async/await
- 📚 **Multi-Store Support**: Parses books from Labirint.ru and IgraSlov.store
- 🛡️ **Robust Error Handling**: Comprehensive error recovery and retry logic
- 📊 **CSV Export**: Clean data export with structured output
- 📝 **Structured Logging**: Detailed logging to both console and rotating log files
- 🔄 **Smart Rate Limiting**: Exponential backoff for rate-limited requests

## Supported Bookstores

- **Labirint** (`labirint.ru`) - Russia's largest online bookstore
- **IgraSlov** (`igraslov.store`) - Specialized Russian bookstore

## Requirements

- Rust 2021 edition or later
- Internet connection for scraping

## Installation

```bash
# Clone the repository
git clone <repository-url>
cd bd_parser

# Build the project
cargo build --release
```

## Usage

### Basic Usage

```bash
# Run with default settings (3 concurrent tasks, 1500 books per store)
cargo run --release
```

### Advanced Usage

```bash
# Customize concurrent tasks and books per store
cargo run --release -- <concurrent_tasks> <books_per_store>

# Example: 5 concurrent tasks, 500 books per store
cargo run --release -- 5 500
```

### Parameters

- `concurrent_tasks` (optional): Number of concurrent parsing tasks
  - Default: 3
  - Must be ≥ 1
  - Higher values = faster but more resource-intensive

- `books_per_store` (optional): Number of books to parse from each store
  - Default: 1500
  - Must be ≥ 1
  - Total books parsed = 2 × books_per_store

## Output

### CSV File

Results are saved to `books.csv` with the following columns:

| Column | Description |
|--------|-------------|
| `site` | Source bookstore (`labirint` or `igra_slov`) |
| `source` | Full URL of the book page |
| `isbn` | International Standard Book Number (ISBN-10 or ISBN-13) |
| `title` | Book title |
| `authors` | Semicolon-separated list of authors |

### Log Files

Logs are saved to `logs/parser.log.<date>` with daily rotation.

Configure log level via the `RUST_LOG` environment variable:

```bash
# Set log level to debug
RUST_LOG=debug cargo run --release

# Set log level for specific modules
RUST_LOG=bd_parser=trace,reqwest=warn cargo run --release
```

## Architecture

### Modules

- **`main.rs`**: Entry point, CLI handling, and orchestration
- **`parse_traits.rs`**: Core data structures (`Book`, `Isbn`, `Author`, `Title`) and `BookParser` trait
- **`labirint.rs`**: Labirint.ru parser implementation
- **`igraslov.rs`**: IgraSlov.store parser implementation
- **`csv_save.rs`**: CSV serialization logic
- **`telemetry.rs`**: Logging and tracing configuration

### Design Patterns

- **Trait-based polymorphism**: `BookParser` trait for site-specific implementations
- **Lazy initialization**: `OnceLock` for HTTP clients and CSS selectors
- **Concurrent streams**: Futures and async/await for parallel processing
- **Type safety**: Newtype patterns for `Isbn`, `Author`, and `Title`

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run integration tests (requires network)
cargo test --release -- --ignored
```

### Linting

```bash
# Run clippy
cargo clippy --all-targets -- -W clippy::all

# Auto-fix issues
cargo clippy --fix
```

### Formatting

```bash
# Check formatting
cargo fmt -- --check

# Apply formatting
cargo fmt
```

## Error Handling

The parser includes robust error handling:

- **Retry Logic**: Automatic retries for transient failures
- **Exponential Backoff**: Smart wait times for rate-limited requests
- **Graceful Degradation**: Continues processing even if some books fail
- **Detailed Errors**: Context-rich error messages for debugging

## Performance Tips

1. **Adjust Concurrency**: Start with 3-5 concurrent tasks, increase if bandwidth allows
2. **Monitor Rate Limits**: Watch logs for 429 responses and reduce concurrency if needed
3. **Use Release Mode**: Always use `--release` for production runs
4. **Check Network**: Ensure stable internet connection for best results

## Known Limitations

- CSS selectors may break if bookstore websites change their HTML structure
- Some books may not have valid ISBNs and will be skipped
- Rate limiting may slow down high-concurrency configurations

## Troubleshooting

### "HTTP 429 Too Many Requests"

Reduce the number of concurrent tasks:

```bash
cargo run --release -- 2 1000
```

### "ISBN not found"

Some books legitimately lack ISBNs. This is expected and logged as a warning.

### Tests Failing

Some integration tests are network-dependent. Run ignored tests separately:

```bash
cargo test -- --ignored
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass and linting is clean
5. Submit a pull request

## License

[Add your license here]

## Acknowledgments

Built with:
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [scraper](https://github.com/causal-agent/scraper) - HTML parsing
- [tokio](https://tokio.rs/) - Async runtime
- [tracing](https://github.com/tokio-rs/tracing) - Structured logging
