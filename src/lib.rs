use std::fmt::Display;

use anyhow::anyhow;
use reqwest::IntoUrl;
#[derive(Debug)]
pub struct Book<T: IntoUrl + Into<String>> {
    author: Vec<Author>,
    isbn: Isbn,
    source: T,
}
type Isbn = String;
type Author = String;
impl<T: IntoUrl + Into<String> + Display + Clone> Book<T> {
    pub async fn new(url: T) -> anyhow::Result<Book<T>> {
        use reqwest::get;
        let page = get(url.clone()).await?.text().await?;
        let html_page = scraper::Html::parse_document(&page);
        let author = parse_author(&html_page, &url).await?;
        let isbn = parse_isbn(&html_page, &url).await?;

        Ok(Self {
            author,
            isbn,
            source: url,
        })
    }
}
async fn parse_isbn<T: Display>(page: &scraper::Html, page_url: T) -> anyhow::Result<Isbn> {
    let isbn_selector =
        scraper::Selector::parse("._right_u86in_12 > div:nth-child(2) > div:nth-child(2)")
            .map_err(|err| anyhow!("bad selector {err}").context(format!("{page_url}")))?;

    match page.select(&isbn_selector).last() {
        Some(elem) => Ok(elem.text().collect::<Isbn>()),
        None => Err(anyhow!("can't find isbn on this page").context(format!("{page_url}"))),
    }
}
async fn parse_author<T: Display>(
    page: &scraper::Html,
    page_url: T,
) -> anyhow::Result<Vec<Author>> {
    let author_selector =
        scraper::Selector::parse("._left_u86in_12 > div:nth-child(1) > div:nth-child(2)")
            .map_err(|err| anyhow!("bad selector {err}").context(format!("{page_url}")))?;

    Ok(page
        .select(&author_selector)
        .map(|node| node.text().collect::<Author>())
        .collect())
}
