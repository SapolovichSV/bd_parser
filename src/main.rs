use std::error::Error;

use quick_xml::de::from_str;
use serde::Deserialize;

use crate::labirint::*;
use crate::parse_traits::{Book, BookParser};
mod labirint;
mod parse_traits;
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
    println!("Hello, world!");
    let resp = reqwest::get(URL1).await?.text().await?;
    let urlset: UrlSet = from_str(&resp)?;
    // println!("{urlset:#?}");
    //

    println!("len: {}", urlset.urls.len());

    let pars = LabirintParser;
    let urls = urlset.urls.iter().take(10);
    for url in urls {
        let book = parse_book_page(&pars, url.loc.clone()).await?;
        println!("{book:#?}");
    }
    // let mut set = JoinSet::new();

    // for url in urlset.urls.iter().take(100) {
    //     let parser = &pars;
    //     let url = url.loc.clone();
    //     parse_book_page(parser, url).await?;
    // }

    Ok(())
}
async fn parse_book_page<T: BookParser>(parser: &T, url: T::Url) -> anyhow::Result<Book<T::Url>> {
    parser.parse_book(url).await
}
