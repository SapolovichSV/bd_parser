//! Parser implementation for IgraSlov.store book store.

use anyhow::anyhow;
use std::{sync::OnceLock, time::Duration};
use tracing::{instrument, warn};

use crate::parse_traits::{self, Author, BookParser, Isbn, Sites, Title};

// CSS selectors for extracting book information
static AUTHOR_SEL_STR: &str = "tr.woocommerce-product-attributes-item:nth-child(1) > td:nth-child(2) > p:nth-child(1) > a:nth-child(1)";
static ISBN_SEL_STR: &str =
    "tr.woocommerce-product-attributes-item:nth-child(7) > td:nth-child(2) > p:nth-child(1)";
static TITLE_SEL_STR: &str = ".single-post-title";

// Lazy-initialized HTTP client and CSS selectors

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static AUTHOR_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static ISBN_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static TITLE_SEL: OnceLock<scraper::Selector> = OnceLock::new();

/// Parser for IgraSlov.store book pages.
pub struct IgraSlov;
impl BookParser for IgraSlov {
    const SITE: parse_traits::Sites = Sites::IgraSlov;

    type Url = String;

    type Context = scraper::Html;
    #[instrument(skip(self), fields(url=%url))]
    async fn fetch(&self, url: &Self::Url) -> anyhow::Result<Self::Context> {
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
        
        let response = client.get(url).send().await?;
        
        if !response.status().is_success() {
            warn!(
                "Unsuccessful response status, likely rate limit: {}",
                response.status()
            );
            return Err(anyhow!("Response status is not success: {}", response.status()));
        }
        
        let body = response.text().await?;
        Ok(scraper::Html::parse_document(&body))
    }

    #[instrument(skip(self, ctx), fields(url=%log_url))]
    async fn parse_authors(
        &self,
        ctx: &Self::Context,
        log_url: &Self::Url,
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
    #[instrument(skip(self, ctx), fields(url=%log_url))]
    async fn parse_isbn(&self, ctx: &Self::Context, log_url: &Self::Url) -> anyhow::Result<Isbn> {
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
    async fn parse_title(&self, ctx: &Self::Context, log_url: &Self::Url) -> anyhow::Result<Title> {
        let book_title_selector = TITLE_SEL
            .get_or_init(|| scraper::Selector::parse(TITLE_SEL_STR).expect("Valid title selector"));
        
        let title_text: String = ctx
            .select(book_title_selector)
            .flat_map(|node| node.text())
            .collect();
        
        Ok(Title::new(title_text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn load_html() -> scraper::Html {
        let html = fs::read_to_string("page_examples/igraslov.html").expect("read igraslov.html");
        scraper::Html::parse_document(&html)
    }

    #[tokio::test]
    async fn parse_authors_from_example() {
        let parser = IgraSlov;
        let ctx = load_html();
        let url = "https://igraslov.store/product/example".to_string();
        let authors = parser.parse_authors(&ctx, &url).await.expect("Authors should be parsed successfully");
        // At least one author-like entry (translator/author cell) should be present in example page
        assert!(!authors.is_empty(), "Expected at least one author");
    }

    #[tokio::test]
    async fn parse_isbn_from_example() {
        let parser = IgraSlov;
        let ctx = load_html();
        let url = "https://igraslov.store/product/example".to_string();
        let isbn = parser.parse_isbn(&ctx, &url).await.expect("ISBN should be parsed successfully");
        let digits = isbn.as_str().chars().filter(|c| c.is_ascii_digit()).count();
        assert!(
            digits == 10 || digits == 13,
            "Expected 10 or 13 ISBN digits, found: {}",
            digits
        );
    }

    #[tokio::test]
    async fn parse_title_from_example() {
        let parser = IgraSlov;
        let ctx = load_html();
        let url = "https://igraslov.store/product/example".to_string();
        let title = parser.parse_title(&ctx, &url).await.expect("Title should be parsed successfully");
        assert!(!title.as_str().trim().is_empty(), "Title should not be empty");
    }

    #[tokio::test]
    async fn parse_isbn_not_found() {
        let parser = IgraSlov;
        let html = scraper::Html::parse_document("<html><body></body></html>");
        let url = "https://igraslov.store/product/example".to_string();
        let result = parser.parse_isbn(&html, &url).await;
        assert!(result.is_err(), "Expected error when ISBN is not found");
    }
}
