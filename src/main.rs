use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use futures::{StreamExt, stream};
use quick_xml::de::from_str;
use serde::Deserialize;

use crate::labirint::*;
use crate::parse_traits::{Book, BookParser};
use tracing::{info, warn};
mod csv_save;
mod igraslov;
mod labirint;
mod parse_traits;
mod telemetry;
use crate::csv_save::{BOOK_CSV_HEADERS, CsvSave};
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
const URL2: [&str; 2] = [
    "https://igraslov.store/product-sitemap.xml",
    "https://igraslov.store/product-sitemap2.xml",
];
static DEFAULT_PARSE_COUNT: usize = 3;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    test().await;
    Ok(())
    // println!("HELP: parser <at_once>");
    // println!("OPTIONAL: <at_once> How much parse at moment, must be >=1");
    // println!("<at_once> default value={DEFAULT_PARSE_COUNT}");
    // let mut parse_count = DEFAULT_PARSE_COUNT;
    // if let Some(num_string) = std::env::args().nth(1)
    //     && let Ok(num) = num_string.parse()
    //     && num >= 1
    // {
    //     parse_count = num;
    // } else if let Some(num_string) = std::env::args().nth(1) {
    //     println!("given <at_once> not a num or less than 1, given: {num_string}");
    // } else {
    //     info!("use default_parse_count:{DEFAULT_PARSE_COUNT}");
    // }
    // let _guard = init_tracing()?;
    // info!(target: "time", "starting parser");
    // let resp = reqwest::get(URL1).await?.text().await?;
    // let urlset: UrlSet = from_str(&resp)?;
    // info!(target: "time", count = urlset.urls.len(), "fetched sitemap urls");

    // let mut wtr = csv::Writer::from_path("books.csv")?;
    // wtr.write_record(BOOK_CSV_HEADERS)?;

    // let how_much_url_process = 20;

    // let urls = urlset
    //     .urls
    //     .into_iter()
    //     .map(|u| u.loc)
    //     .filter(|u| u.contains("/books/"))
    //     .take(how_much_url_process);

    // let counter = Arc::new(AtomicU64::new(0));
    // let books: Vec<_> = stream::iter(urls)
    //     .map(|url| {
    //         let counter = Arc::clone(&counter);
    //         async move {
    //             let result = parse_book_page(&LabirintParser, url).await;
    //             let processed = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    //             println!("processed: {processed}/2000");
    //             result
    //         }
    //     })
    //     .buffer_unordered(parse_count)
    //     .collect()
    //     .await;
    // for book in books.iter() {
    //     match book {
    //         Ok(book) => {
    //             info!("succesfull parsed book with url {}", book.source);
    //             book.write_csv_record(&mut wtr)?
    //         }
    //         Err(e) => warn!("book unsuccesfull parse {e}"),
    //     }
    // }

    // wtr.flush()?;
    // Ok(())
}
async fn test() {
    let resps = {
        let mut resps = vec![];
        for url in URL2 {
            let resp = reqwest::get(url)
                .await
                .expect("WTF")
                .text()
                .await
                .expect("WTF2");
            resps.push(resp);
        }
        resps
    };
    println!("{}", resps.first().expect(""));
    use scraper;
    let html_first = scraper::Html::parse_document(resps.first().expect("must be"));
}
#[tracing::instrument(skip(parser), fields(url=%url))]
async fn parse_book_page<T: BookParser>(parser: &T, url: T::Url) -> anyhow::Result<Book<T::Url>> {
    parser.parse_book(url).await
}
