use std::error::Error;

use quick_xml::de::from_str;
use serde::Deserialize;

use crate::labirint::*;
use crate::parse_traits::{Book, BookParser};
use tracing::info;
mod labirint;
mod parse_traits;
mod telemetry;
use crate::telemetry::init_tracing;
#[derive(Debug, Deserialize)]
struct BookUrl {
    loc: String,
}
#[derive(Debug, Deserialize)]
struct UrlSet {
    #[serde(rename = "url")]
    urls: Vec<BookUrl>,
}
const URL1: &str = "https://www.labirint.ru/smcatalog2.xml";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _guard = init_tracing()?;
    info!(target: "time", "starting parser");
    let resp = reqwest::get(URL1).await?.text().await?;
    let urlset: UrlSet = from_str(&resp)?;
    info!(target: "time", count = urlset.urls.len(), "fetched sitemap urls");

    let pars = LabirintParser;
    let urls = urlset.urls.iter().take(10);
    for url in urls {
        info!(target: "time", url = %url.loc, "parsing book page");
        let book = parse_book_page(&pars, url.loc.clone()).await?;
        info!(target: "time", book = ?book, "parsed book");
    }

    Ok(())
}
#[tracing::instrument(skip(parser), fields(url=%url))]
async fn parse_book_page<T: BookParser>(parser: &T, url: T::Url) -> anyhow::Result<Book<T::Url>> {
    parser.parse_book(url).await
}
