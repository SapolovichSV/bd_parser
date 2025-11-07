use anyhow::anyhow;
use std::{sync::OnceLock, time::Duration};
use tracing::{instrument, warn};

use crate::parse_traits::{self, Author, BookParser, Isbn, Sites, Title};
static AUTHOR_SEL_STR: &str = "tr.woocommerce-product-attributes-item:nth-child(1) > td:nth-child(2) > p:nth-child(1) > a:nth-child(1)";
static ISBN_SEL_STR: &str =
    "tr.woocommerce-product-attributes-item:nth-child(7) > td:nth-child(2) > p:nth-child(1)";
static TITLE_SEL_STR: &str = ".single-post-title";

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static AUTHOR_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static ISBN_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static TITLE_SEL: OnceLock<scraper::Selector> = OnceLock::new();
pub struct IgraSlov;
impl BookParser for IgraSlov {
    const SITE: parse_traits::Sites = Sites::IgraSlov;

    type Url = String;

    type Context = scraper::Html;
    #[instrument(skip(self),fields(url=%url))]
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
                .expect("http client")
        });
        match client.get(url).send().await {
            Ok(response) if !response.status().is_success() => {
                warn!(
                    "bad status code probably rate limit code: {}",
                    response.status()
                );
                return Err(anyhow!("response status is not success"));
            }
            Ok(response) => {
                let resp = response.text().await?;
                Ok(scraper::Html::parse_document(&resp))
            }
            Err(e) => return Err(e.into()),
        }
    }

    #[instrument(skip(self,ctx),fields(url=%log_url))]
    async fn parse_authors(
        &self,
        ctx: &Self::Context,
        log_url: &Self::Url,
    ) -> anyhow::Result<Vec<Author>> {
        let author_selector = AUTHOR_SEL
            .get_or_init(|| scraper::Selector::parse(AUTHOR_SEL_STR).expect("author selector"));

        Ok(ctx
            .select(author_selector)
            .map(|node| Author::new(node.text().collect::<String>()))
            .collect())
    }
    #[instrument(skip(self, ctx, _log_url))]
    // TODO fix selector https://igraslov.store/product/serebryakov-a-fistula-gorodets-klap/
    async fn parse_isbn(&self, ctx: &Self::Context, _log_url: &Self::Url) -> anyhow::Result<Isbn> {
        let isbn_selector =
            ISBN_SEL.get_or_init(|| scraper::Selector::parse(ISBN_SEL_STR).expect("isbn selector"));

        match ctx.select(isbn_selector).next_back() {
            Some(elem) => {
                let raw: String = elem.text().collect::<String>().replace("\u{a0}", "");
                match Isbn::try_from(raw) {
                    Ok(isbn) => Ok(isbn),
                    Err(e) => {
                        warn!("can't parse isbn:{e}");
                        Err(anyhow!("can't parse isbn"))
                    }
                }
            }
            None => {
                warn!(target: "time","ISBN not found on page");
                Err(anyhow!("can't find isbn on this page"))
            }
        }
    }

    #[instrument(skip(self,ctx),fields(url=%log_url))]
    async fn parse_title(&self, ctx: &Self::Context, log_url: &Self::Url) -> anyhow::Result<Title> {
        let book_title_selector = TITLE_SEL
            .get_or_init(|| scraper::Selector::parse(TITLE_SEL_STR).expect("title selector"));
        Ok(Title::new(
            ctx.select(book_title_selector)
                .map(|node| node.text().collect::<String>())
                .collect::<String>(),
        ))
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
        let authors = parser.parse_authors(&ctx, &url).await.expect("authors ok");
        // At least one author-like entry (translator/author cell) should be present in example page
        assert!(!authors.is_empty());
    }

    #[tokio::test]
    async fn parse_isbn_from_example() {
        let parser = IgraSlov;
        let ctx = load_html();
        let url = "https://igraslov.store/product/example".to_string();
        let isbn = parser.parse_isbn(&ctx, &url).await.expect("isbn ok");
        let digits = isbn.as_str().chars().filter(|c| c.is_ascii_digit()).count();
        assert!(
            digits == 10 || digits == 13,
            "unexpected isbn digits count: {}",
            digits
        );
    }

    #[tokio::test]
    async fn parse_title_from_example() {
        let parser = IgraSlov;
        let ctx = load_html();
        let url = "https://igraslov.store/product/example".to_string();
        let title = parser.parse_title(&ctx, &url).await.expect("title ok");
        assert!(!title.as_str().trim().is_empty());
    }

    #[tokio::test]
    async fn parse_isbn_not_found() {
        let parser = IgraSlov;
        let html = scraper::Html::parse_document("<html><body></body></html>");
        let url = "https://igraslov.store/product/example".to_string();
        let res = parser.parse_isbn(&html, &url).await;
        assert!(res.is_err());
    }
}
