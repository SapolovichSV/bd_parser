use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use anyhow::{Context, anyhow};
use futures::{StreamExt, stream};
use quick_xml::de::from_str;
use serde::Deserialize;

use crate::igraslov::IgraSlov;
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
static PARSE_FROM_ONE_SITE: usize = 1500;
async fn parse_sitemap_igraslov(sitemap: &str) -> anyhow::Result<Vec<String>> {
    let resp = reqwest::get(sitemap)
        .await
        .context("GET igraslov sitemap failed")?
        .error_for_status()
        .context("non-success status for igraslov sitemap")?
        .text()
        .await
        .context("reading igraslov sitemap body failed")?;
    let html = scraper::Html::parse_document(&resp);
    let selector = scraper::Selector::parse("loc").expect("should");
    let elems = html.select(&selector);

    let mut books_urls = vec![];
    static BOOK_INDICATORS: [&str; 4] = ["tvyord", "klap", "myagk", "super"];

    for (i, nodes) in elems.enumerate() {
        if let Some(noderef) = nodes.first_child() {
            let url_comment = noderef
                .value()
                .as_comment()
                .ok_or_else(|| anyhow!("expected comment node for <loc>"))?;
            let mut url: String = (*url_comment).parse()?;
            url = url
                .replace("[CDATA[", "")
                .replace("]]", "")
                .trim()
                .to_string();
            books_urls.push(url);
        } else {
            warn!(index = i, "can't fetch url node");
        }
    }
    info!("fetched url's from igraslov sitemap");
    Ok(books_urls
        .into_iter()
        .filter(|url| BOOK_INDICATORS.iter().any(|pat| url.contains(pat)))
        .collect())
}
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("HELP: parser <at_once> <how_much_from_one_store");
    println!("OPTIONAL: <at_once> How much parse at moment, must be >=1");
    println!("OPTIONAL: <how_much_from_one_store>, must be >=1");
    println!("<at_once> default value={DEFAULT_PARSE_COUNT}");
    println!("<how_much_from_one_store default value = {PARSE_FROM_ONE_SITE}");
    let mut parse_count = DEFAULT_PARSE_COUNT;
    let mut parse_from_one = PARSE_FROM_ONE_SITE;
    for (i, arg) in std::env::args().skip(1).enumerate() {
        let (processing, name_var) = match i {
            0 => (&mut parse_count, "<at_once>"),
            1 => (&mut parse_from_one, "<how_much_from_one_store>"),
            _ => return Err(anyhow!("too much env args")),
        };
        let num: usize = arg.parse()?;
        if num >= 1 {
            *processing = num
        } else {
            return Err(anyhow!("given {name_var} is not a num or < 1"));
        }
        info!("{name_var} value = {}", *processing);
    }
    let _guard = init_tracing().map_err(|e| anyhow!("{e}"))?;
    info!(target: "time", "starting parser");
    let resp = reqwest::get(URL1).await?.text().await?;
    let urlset: UrlSet = from_str(&resp)?;
    info!(target: "time", count = urlset.urls.len(), "fetched sitemap urls");

    let mut wtr = csv::Writer::from_path("books.csv")?;
    wtr.write_record(BOOK_CSV_HEADERS)?;

    let how_much_url_process_at_once_source = 20;

    let urls_labirint: Vec<String> = urlset
        .urls
        .into_iter()
        .map(|u| u.loc)
        .filter(|u| u.contains("/books/"))
        .take(how_much_url_process_at_once_source)
        .collect();
    let urls_igraslov: Vec<String> = {
        let mut books: Vec<String> = vec![];
        if parse_from_one > 1000 {
            let mut first_part = parse_sitemap_igraslov(URL2[0]).await?;
            let mut second_part = parse_sitemap_igraslov(URL2[1]).await?;
            books.append(&mut first_part);
            books.append(&mut second_part);
        } else {
            books.append(&mut parse_sitemap_igraslov(URL2[0]).await?);
        }
        books
    }
    .into_iter()
    .take(how_much_url_process_at_once_source)
    .collect();
    let urls: Vec<String> = interleave(urls_igraslov.clone().into_iter(), urls_labirint.clone().into_iter()).collect();

    let total = urls.len() as u64;

    let counter = Arc::new(AtomicU64::new(0));
    let books: Vec<_> = stream::iter(urls)
        .map(|url| {
            let counter = Arc::clone(&counter);
            async move {
                let result;
                if url.contains("labirint") {
                    result = parse_book_page(&LabirintParser, url).await;
                } else if url.contains("igraslov") {
                    result = parse_book_page(&IgraSlov, url).await;
                } else {
                    todo!()
                }
                let processed = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                println!("processed: {processed}/{total}");
                result
            }
        })
        .buffer_unordered(parse_count)
        .collect()
        .await;
    for book in books.iter() {
        match book {
            Ok(book) => {
                info!("succesfull parsed book with url {}", book.source);
                book.write_csv_record(&mut wtr)?
            }
            Err(e) => warn!("book unsuccesfull parse {e}"),
        }
    }

    wtr.flush()?;
    Ok(())
}
#[tracing::instrument(skip(parser), fields(url=%url))]
async fn parse_book_page<T: BookParser>(parser: &T, url: T::Url) -> anyhow::Result<Book<T::Url>> {
    parser.parse_book(url).await
}
fn interleave<I, J, T>(mut a: I, mut b: J) -> impl Iterator<Item = T>
where
    I: Iterator<Item = T>,
    J: Iterator<Item = T>,
{
    let mut take_from_a = true;

    std::iter::from_fn(move || {
        let next = if take_from_a {
            a.next().or_else(|| b.next())
        } else {
            b.next().or_else(|| a.next())
        };
        take_from_a = !take_from_a;
        next
    })
}
