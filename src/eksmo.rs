use std::{sync::OnceLock, time::Duration};

use anyhow::anyhow;
use tracing::{instrument, warn};

use crate::parse_traits::{Author, BookParser, Isbn, Sites, Title};

static AUTHOR_SEL_STR: &str = ".book-page__card-author-link";
static ISBN_SEL_STR: &str = "span.copy__val";
static TITLE_SEL_STR: &str = ".book-page__card-title";
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static AUTHOR_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static ISBN_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static TITLE_SEL: OnceLock<scraper::Selector> = OnceLock::new();
pub struct EksmoParser;
impl BookParser for EksmoParser {
    const SITE: crate::parse_traits::Sites = Sites::Eksmo;

    type Url = String;

    type Context = scraper::Html;

    #[instrument(skip(self, url))]
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
    #[instrument(skip(self, ctx, _log_url))]
    async fn parse_authors(
        &self,
        ctx: &Self::Context,
        _log_url: &Self::Url,
    ) -> anyhow::Result<Vec<crate::parse_traits::Author>> {
        let author_selector = AUTHOR_SEL
            .get_or_init(|| scraper::Selector::parse(AUTHOR_SEL_STR).expect("author selector"));

        Ok(ctx
            .select(author_selector)
            .map(|node| Author::new(node.text().collect::<String>()))
            .collect())
    }

    #[instrument(skip(self, ctx, _log_url))]
    async fn parse_isbn(
        &self,
        ctx: &Self::Context,
        _log_url: &Self::Url,
    ) -> anyhow::Result<crate::parse_traits::Isbn> {
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
                warn!(target: "time","ISBN not found on page{_log_url}");
                Err(anyhow!("can't find isbn on this page"))
            }
        }
    }

    #[instrument(skip(self, ctx, _log_url))]
    async fn parse_title(
        &self,
        ctx: &Self::Context,
        _log_url: &Self::Url,
    ) -> anyhow::Result<crate::parse_traits::Title> {
        let book_title_selector = TITLE_SEL
            .get_or_init(|| scraper::Selector::parse(TITLE_SEL_STR).expect("title selector"));
        let title = {
            ctx.select(book_title_selector)
                .map(|node| node.text().collect::<String>())
                .collect::<String>()
        };
        Ok(Title::new(title))
    }

    #[instrument(skip(self),fields(url=&url))]
    async fn parse_book(
        &self,
        url: Self::Url,
    ) -> anyhow::Result<crate::parse_traits::Book<Self::Url>> {
        let ctx = self.fetch(&url).await?;
        let authors = self.parse_authors(&ctx, &url).await?;
        let title = self.parse_title(&ctx, &url).await?;
        let isbn = self.parse_isbn(&ctx, &url).await?;
        Ok(crate::parse_traits::Book {
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

    const EXPECTED_AUTHOR: &str = "Андрей Самарин";
    const EXPECTED_ISBN: &str = "978-5-04-156838-2";
    const EXPECTED_TITLE: &str =
        "Структура таланта. От иллюзий к реальности: как стать настоящим художником";

    fn get_context() -> scraper::Html {
        let context = include_str!("../page_examples/eksmo.html");
        scraper::Html::parse_document(context)
    }

    #[tokio::test]
    async fn parse_authors_ok() {
        let parser = EksmoParser;
        let ctx = get_context();
        let url = "https://eksmo.ru/book/example".to_string();
        let authors = parser
            .parse_authors(&ctx, &url)
            .await
            .expect("authors parsed");
        assert!(!authors.is_empty());
        assert_eq!(authors[0].as_str(), EXPECTED_AUTHOR);
    }

    #[tokio::test]
    async fn parse_isbn_ok() {
        let parser = EksmoParser;
        let ctx = get_context();
        let url = "https://eksmo.ru/book/example".to_string();
        let isbn = parser.parse_isbn(&ctx, &url).await.expect("isbn parsed");
        assert_eq!(isbn.as_str(), EXPECTED_ISBN);
        let digits = isbn.as_str().chars().filter(|c| c.is_ascii_digit()).count();
        assert_eq!(digits, 13);
    }

    #[tokio::test]
    async fn parse_title_ok() {
        let parser = EksmoParser;
        let ctx = get_context();
        let url = "https://eksmo.ru/book/example".to_string();
        let title = parser.parse_title(&ctx, &url).await.expect("title parsed");
        assert_eq!(title.as_str(), EXPECTED_TITLE);
    }

    #[tokio::test]
    async fn parse_isbn_not_found() {
        let parser = EksmoParser;
        let empty_ctx = scraper::Html::parse_document("<html><body></body></html>");
        let url = "https://eksmo.ru/book/example".to_string();
        let res = parser.parse_isbn(&empty_ctx, &url).await;
        assert!(res.is_err());
    }
}
