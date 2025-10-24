use parser::Book;
use std::error::Error;

use quick_xml::de::from_str;
use reqwest;
use scraper::{Html, Selector};
use serde::Deserialize;
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

    println!("parsing book with url: {}", urlset.urls[0].loc.clone());

    let books: Vec<String> = (0..3).map(|i| urlset.urls[i].loc.clone()).collect();
    let books = get_books(books).await?;
    println!("{books:?}");

    Ok(())
}
async fn get_books(urls: Vec<String>) -> anyhow::Result<Vec<Book<String>>> {
    let mut books = vec![];
    for url in urls {
        books.push(Book::new(url).await?);
    }
    Ok(books)
}
