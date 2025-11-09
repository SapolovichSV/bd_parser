use std::sync::OnceLock;

use crate::parse_traits::{BookParser, Sites};

static AUTHOR_SEL_STR: &str = "";
static ISBN_SEL_STR: &str = "";
static TITLE_SEL_STR: &str = "";
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static AUTHOR_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static ISBN_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static TITLE_SEL: OnceLock<scraper::Selector> = OnceLock::new();
pub struct EksmoParser;
impl BookParser for EksmoParser {
    const SITE: crate::parse_traits::Sites = Sites::Eksmo;

    type Url = String;

    type Context = scraper::Html;

    async fn fetch(&self, url: &Self::Url) -> anyhow::Result<Self::Context> {
        todo!()
    }

    async fn parse_authors(
        &self,
        ctx: &Self::Context,
        log_url: &Self::Url,
    ) -> anyhow::Result<Vec<crate::parse_traits::Author>> {
        todo!()
    }

    async fn parse_isbn(
        &self,
        ctx: &Self::Context,
        log_url: &Self::Url,
    ) -> anyhow::Result<crate::parse_traits::Isbn> {
        todo!()
    }

    async fn parse_title(
        &self,
        ctx: &Self::Context,
        log_url: &Self::Url,
    ) -> anyhow::Result<crate::parse_traits::Title> {
        todo!()
    }

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
