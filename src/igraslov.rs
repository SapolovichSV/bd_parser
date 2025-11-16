use anyhow::{Context, anyhow};
use std::{sync::OnceLock, time::Duration};
use tracing::{debug, instrument, warn};

use crate::parse_traits::{self, Author, BookParser, Description, Isbn, Price, Sites, Title};
static AUTHOR_SEL_STR: &str = "tr.woocommerce-product-attributes-item:nth-child(1) > td:nth-child(2) > p:nth-child(1) > a:nth-child(1)";
static ISBN_SEL_STR: &str = "tr.woocommerce-product-attributes-item--attribute_pa_isbn-issn-1 td p";
static TITLE_SEL_STR: &str = ".single-post-title";
static DESCR_SEL_STR: &str = ".woocommerce-product-details__short-description > p:nth-child(1)";
static PRICE_SEL_STR: &str = "p.price > span:nth-child(1) > bdi:nth-child(1)";

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static AUTHOR_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static ISBN_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static TITLE_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static DESCR_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static PRICE_SEL: OnceLock<scraper::Selector> = OnceLock::new();
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
                warn!(target: "time","ISBN not found on page {_log_url}");
                Err(anyhow!("can't find isbn on this page"))
            }
        }
    }

    #[instrument(skip(self,ctx),fields(url=%log_url))]
    async fn parse_title(&self, ctx: &Self::Context, log_url: &Self::Url) -> anyhow::Result<Title> {
        let book_title_selector = TITLE_SEL
            .get_or_init(|| scraper::Selector::parse(TITLE_SEL_STR).expect("title selector"));
        let title = {
            let mut title = ctx
                .select(book_title_selector)
                .map(|node| node.text().collect::<String>())
                .collect::<String>();
            if let Some(striped) = title.strip_prefix("_") {
                title = striped.to_string();
            }
            title
        };
        Ok(Title::new(title))
    }

    #[instrument(skip(self, ctx))]
    async fn parse_description(
        &self,
        ctx: &Self::Context,
    ) -> anyhow::Result<crate::parse_traits::Description> {
        let book_descr_sel = DESCR_SEL
            .get_or_init(|| scraper::Selector::parse(DESCR_SEL_STR).expect("descr selector"));
        let descr = ctx
            .select(book_descr_sel)
            .map(|node| node.text().collect::<String>())
            .collect();
        Ok(Description::new(descr))
    }

    async fn parse_price(&self, ctx: &Self::Context) -> anyhow::Result<parse_traits::Price> {
        let price_sel = PRICE_SEL
            .get_or_init(|| scraper::Selector::parse(PRICE_SEL_STR).expect("price selector"));
        let mut price_string: String = match ctx.select(price_sel).next_back() {
            Some(elref) => elref.text().collect(),
            None => return Err(anyhow!("can't parse price")),
        };
        let forbidden_symb = [',', '\u{a0}', '₽'];
        price_string.retain(|x| !forbidden_symb.contains(&x));
        debug!(price_string);
        let price = match price_string.parse() {
            Ok(price) => price,
            Err(e) => {
                warn!("can't parse price : {e}");
                return Err(e);
            }
        };
        Ok(price)
    }
    // #[instrument(skip(self),fields(url=&url))]
    // async fn parse_book(&self, url: Self::Url) -> anyhow::Result<parse_traits::Book<Self::Url>> {
    //     let ctx = self.fetch(&url).await?;
    //     let authors = self
    //         .parse_authors(&ctx, &url)
    //         .await
    //         .with_context(|| format!("fetch failed: {}", url))?;
    //     let title = self
    //         .parse_title(&ctx, &url)
    //         .await
    //         .with_context(|| format!("fetch failed: {}", url))?;
    //     let isbn = self
    //         .parse_isbn(&ctx, &url)
    //         .await
    //         .with_context(|| format!("fetch failed: {}", url))?;
    //     let description = self
    //         .parse_description(&ctx)
    //         .await
    //         .with_context(|| format!("fetch failed: {}", url))?;
    //     let price = self
    //         .parse_price(&ctx)
    //         .await
    //         .with_context(|| format!("fetch failed: {}", url))?;
    //     Ok(parse_traits::Book {
    //         authors,
    //         isbn,
    //         source: url,
    //         title,
    //         site: Self::SITE,
    //         description,
    //         price,
    //     })
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    const EXPECTED_PRICE: u128 = 89500;

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
    #[tokio::test]
    async fn parse_descr() {
        let parser = IgraSlov;
        let ctx = load_html();
        let url = "https://igraslov.store/product/example".to_string();
        let descr = parser.parse_description(&ctx).await.expect("should");
        assert!(descr.as_str().len() > 10);
    }
    #[tokio::test]
    async fn parse_price() {
        let parser = IgraSlov;
        let ctx = load_html();
        let price = parser.parse_price(&ctx).await.expect("should be");
        assert_eq!(u128::from(price), EXPECTED_PRICE);
    }
}
