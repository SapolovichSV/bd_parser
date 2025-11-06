//! Parser implementation for Labirint.ru book store.

use crate::parse_traits::{self, Author, BookParser, Isbn, Sites, Title};
use anyhow::anyhow;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{info, instrument, warn};

// CSS selectors for extracting book information
static AUTHOR_SEL_STR: &str = "._left_u86in_12 > div:nth-child(1) > div:nth-child(2)";
static ISBN_SEL_STR: &str = "._right_u86in_12 > div:nth-child(2) > div:nth-child(2)";
static TITLE_SEL_STR: &str = "._h1_5o36c_18";

// Lazy-initialized HTTP client and CSS selectors
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static AUTHOR_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static ISBN_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static TITLE_SEL: OnceLock<scraper::Selector> = OnceLock::new();

/// Maximum number of retry attempts for failed HTTP requests.
const MAX_RETRIES: u8 = 1;

/// Parser for Labirint.ru book pages.
pub struct LabirintParser;
impl BookParser for LabirintParser {
    const SITE: parse_traits::Sites = Sites::Labirint;
    type Url = String;

    type Context = scraper::Html;

    #[instrument(skip(self), fields(url=%url))]
    async fn fetch(&self, url: &Self::Url) -> anyhow::Result<Self::Context> {
        if !url.contains("books") {
            warn!(target: "time", "Rejected non-book URL");
            return Err(anyhow!("Non-book URL rejected"));
        }
        
        let client = CLIENT.get_or_init(|| {
            reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(15))
                .pool_max_idle_per_host(4)
                .tcp_keepalive(Some(Duration::from_secs(30)))
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .expect("Failed to build HTTP client")
        });

        let mut last_err: Option<reqwest::Error> = None;
        let mut last_status: Option<reqwest::StatusCode> = None;
        
        for attempt in 0..=MAX_RETRIES {
            match client.get(url).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let body = resp.text().await?;
                        return Ok(scraper::Html::parse_document(&body));
                    }
                    
                    last_status = Some(status);
                    
                    // Only retry on rate limit or server errors
                    if (status.as_u16() == 429 || status.is_server_error()) && attempt < MAX_RETRIES {
                        let retry_after = resp
                            .headers()
                            .get(reqwest::header::RETRY_AFTER)
                            .and_then(|h| h.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or_else(|| 2u64.pow(attempt as u32).min(8));
                        
                        warn!(
                            target: "time",
                            attempt,
                            %status,
                            wait = retry_after,
                            "Retrying after delay due to rate limit or server error"
                        );
                        tokio::time::sleep(Duration::from_secs(retry_after)).await;
                        continue;
                    }
                    
                    return Err(anyhow!("HTTP error: {}", status));
                }
                Err(e) => {
                    last_err = Some(e);
                    if attempt < MAX_RETRIES {
                        let wait = 2u64.pow(attempt as u32).min(8);
                        warn!(
                            target: "time",
                            attempt,
                            wait,
                            "Network error, retrying after delay"
                        );
                        tokio::time::sleep(Duration::from_secs(wait)).await;
                        continue;
                    }
                }
            }
        }
        
        if let Some(status) = last_status {
            Err(anyhow!("HTTP error after retries: {}", status))
        } else {
            Err(anyhow!("Network error after retries: {}", last_err.unwrap()))
        }
    }

    #[instrument(skip(self, ctx), fields(url=%url))]
    async fn parse_authors(
        &self,
        ctx: &Self::Context,
        url: &Self::Url,
    ) -> anyhow::Result<Vec<Author>> {
        let author_selector = AUTHOR_SEL
            .get_or_init(|| scraper::Selector::parse(AUTHOR_SEL_STR).expect("Valid author selector"));

        Ok(ctx
            .select(author_selector)
            .map(|node| {
                let text: String = node.text().collect();
                Author::new(text)
            })
            .collect())
    }

    #[instrument(skip(self, ctx), fields(url=%url))]
    async fn parse_isbn(&self, ctx: &scraper::Html, url: &Self::Url) -> anyhow::Result<Isbn> {
        let isbn_selector =
            ISBN_SEL.get_or_init(|| scraper::Selector::parse(ISBN_SEL_STR).expect("Valid ISBN selector"));

        match ctx.select(isbn_selector).next_back() {
            Some(elem) => {
                let raw: String = elem.text().collect::<String>().replace('\u{a0}', "");
                Isbn::try_from(raw).map_err(|e| {
                    warn!("Failed to parse ISBN: {}", e);
                    anyhow!("ISBN parsing failed")
                })
            }
            None => {
                warn!(target: "time", "ISBN not found on page");
                Err(anyhow!("ISBN not found on page"))
            }
        }
    }
    #[instrument(skip(self, ctx), fields(url=%log_url))]
    async fn parse_title(
        &self,
        ctx: &Self::Context,
        log_url: &Self::Url,
    ) -> anyhow::Result<parse_traits::Title> {
        let book_title_selector = TITLE_SEL
            .get_or_init(|| scraper::Selector::parse(TITLE_SEL_STR).expect("Valid title selector"));
        
        let title_text: String = ctx
            .select(book_title_selector)
            .flat_map(|node| node.text())
            .collect();
        
        Ok(Title::new(title_text))
    }
    #[instrument(skip(self), fields(url=%url))]
    async fn parse_book(&self, url: Self::Url) -> anyhow::Result<parse_traits::Book<Self::Url>> {
        info!(target: "time", "Starting book processing");
        let ctx = self.fetch(&url).await?;
        let authors = self.parse_authors(&ctx, &url).await?;
        let title = self.parse_title(&ctx, &url).await?;
        let isbn = self.parse_isbn(&ctx, &url).await?;
        info!(target: "time", "Completed book processing");
        Ok(parse_traits::Book {
            authors,
            isbn,
            source: url,
            title,
            site: Self::SITE,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_HTML: &str = r#"
<!DOCTYPE html>
<html>
<body>
    <div class="_left_u86in_12">
        <div>
            <div>Автор:</div>
            <div>Лев Толстой</div>
        </div>
    </div>
    <div class="_right_u86in_12">
        <div>Placeholder</div>
        <div>
            <div>ISBN Label</div>
            <div>978-5-17-123456-7</div>
        </div>
    </div>
    <h1 class="_h1_5o36c_18">Война и мир</h1>
</body>
</html>
"#;

    const TEST_URL: &str = "https://www.labirint.ru/books/123456/";
    const EXPECTED_TITLE: &str = "Война и мир";
    const EXPECTED_AUTHOR: &str = "Лев Толстой";

    fn create_test_context() -> scraper::Html {
        scraper::Html::parse_document(TEST_HTML)
    }

    #[tokio::test]
    async fn test_parse_authors() {
        let parser = LabirintParser;
        let ctx = create_test_context();
        let url = TEST_URL.to_string();

        let result = parser.parse_authors(&ctx, &url).await;
        assert!(result.is_ok(), "parse_authors failed: {:?}", result.err());

        let authors = result.unwrap();
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].as_str(), EXPECTED_AUTHOR);
    }

    #[tokio::test]
    async fn test_parse_isbn() {
        let parser = LabirintParser;
        let ctx = create_test_context();
        let url = TEST_URL.to_string();

        let result = parser.parse_isbn(&ctx, &url).await;
        assert!(result.is_ok(), "parse_isbn failed: {:?}", result.err());

        let isbn = result.unwrap();
        // The ISBN is stored with its original formatting (with dashes)
        assert_eq!(isbn.as_str(), "978-5-17-123456-7");
    }

    #[tokio::test]
    async fn test_parse_title() {
        let parser = LabirintParser;
        let ctx = create_test_context();
        let url = TEST_URL.to_string();

        let result = parser.parse_title(&ctx, &url).await;
        assert!(result.is_ok());

        let title = result.unwrap();
        assert_eq!(title.as_str(), EXPECTED_TITLE);
    }

    #[tokio::test]
    async fn test_parse_isbn_not_found() {
        let parser = LabirintParser;
        let html = scraper::Html::parse_document("<html><body></body></html>");
        let url = TEST_URL.to_string();

        let result = parser.parse_isbn(&html, &url).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_invalid_url() {
        let parser = LabirintParser;
        let invalid_url = "https://www.labirint.ru/invalid/".to_string();

        let result = parser.fetch(&invalid_url).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "Integration test - requires network and may fail due to rate limiting"]
    async fn test_parse_book_integration() {
        let parser = LabirintParser;
        let url = "https://www.labirint.ru/books/801841/".to_string();

        let result = parser.parse_book(url).await;
        assert!(result.is_ok(), "Integration test failed: {:?}", result.err());

        let book = result.unwrap();
        assert!(!book.authors.is_empty());
        assert!(!book.isbn.as_str().is_empty());
        assert!(!book.title.as_str().is_empty());
    }
}
