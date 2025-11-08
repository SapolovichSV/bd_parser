//! Book parser that scrapes book information from Russian bookstore websites.
//!
//! This application fetches book data (title, author, ISBN) from:
//! - Labirint.ru
//! - IgraSlov.store
//!
//! Results are saved to a CSV file.

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

/// XML structure for sitemap URL entry
#[derive(Debug, Deserialize)]
struct BookUrl {
    loc: String,
}

/// XML structure for sitemap URL set
#[derive(Debug, Deserialize)]
struct UrlSet {
    #[serde(rename = "url")]
    urls: Vec<BookUrl>,
}

// Configuration constants
const LABIRINT_SITEMAP_URL: &str = "https://www.labirint.ru/smcatalog2.xml";
const IGRASLOV_SITEMAP_URLS: [&str; 2] = [
    "https://igraslov.store/product-sitemap.xml",
    "https://igraslov.store/product-sitemap2.xml",
];

/// Default number of concurrent parsing tasks
const DEFAULT_CONCURRENT_TASKS: usize = 3;

/// Default number of books to parse from each bookstore
const DEFAULT_BOOKS_PER_STORE: usize = 1500;

/// Threshold for parsing from second IgraSlov sitemap
const IGRASLOV_SECOND_SITEMAP_THRESHOLD: usize = 1000;

/// Parses an IgraSlov sitemap XML to extract book product URLs.
///
/// Filters URLs to only include those with book-related indicators in the path.
///
/// # Errors
///
/// Returns an error if:
/// - HTTP request fails
/// - XML parsing fails
/// - Unexpected HTML structure
async fn parse_sitemap_igraslov(sitemap: &str) -> anyhow::Result<Vec<String>> {
    let resp = reqwest::get(sitemap)
        .await
        .context("Failed to GET IgraSlov sitemap")?
        .error_for_status()
        .context("Non-success HTTP status for IgraSlov sitemap")?
        .text()
        .await
        .context("Failed to read IgraSlov sitemap body")?;
    
    let html = scraper::Html::parse_document(&resp);
    let selector = scraper::Selector::parse("loc")
        .expect("Invalid CSS selector for <loc>");
    let elems = html.select(&selector);

    let mut books_urls = Vec::new();
    
    // Book cover type indicators in URLs
    const BOOK_INDICATORS: [&str; 4] = ["tvyord", "klap", "myagk", "super"];

    for (i, nodes) in elems.enumerate() {
        if let Some(noderef) = nodes.first_child() {
            let url_comment = noderef
                .value()
                .as_comment()
                .ok_or_else(|| anyhow!("Expected comment node for <loc> at index {}", i))?;
            
            let url: String = (*url_comment)
                .parse()
                .context("Failed to parse URL from comment")?;
            
            let cleaned_url = url
                .replace("[CDATA[", "")
                .replace("]]", "")
                .trim()
                .to_string();
            
            books_urls.push(cleaned_url);
        } else {
            warn!(index = i, "Cannot fetch URL node - no first child");
        }
    }
    
    info!("Fetched {} URLs from IgraSlov sitemap", books_urls.len());
    
    Ok(books_urls
        .into_iter()
        .filter(|url| BOOK_INDICATORS.iter().any(|pat| url.contains(pat)))
        .collect())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing/logging
    let _guard = init_tracing()
        .map_err(|e| anyhow!("Failed to initialize logging: {}", e))?;
    
    // Parse command-line arguments
    println!("USAGE: parser [concurrent_tasks] [books_per_store]");
    println!("  concurrent_tasks: Number of concurrent parsing tasks (default: {})", DEFAULT_CONCURRENT_TASKS);
    println!("  books_per_store: Number of books to parse from each store (default: {})", DEFAULT_BOOKS_PER_STORE);
    
    let mut concurrent_tasks = DEFAULT_CONCURRENT_TASKS;
    let mut books_per_store = DEFAULT_BOOKS_PER_STORE;
    
    for (i, arg) in std::env::args().skip(1).enumerate() {
        let (target_var, var_name) = match i {
            0 => (&mut concurrent_tasks, "concurrent_tasks"),
            1 => (&mut books_per_store, "books_per_store"),
            _ => return Err(anyhow!("Too many command-line arguments provided")),
        };
        
        let num: usize = arg.parse()
            .context(format!("Invalid number for {}", var_name))?;
        
        if num < 1 {
            return Err(anyhow!("{} must be >= 1, got {}", var_name, num));
        }
        
        *target_var = num;
        info!("{} = {}", var_name, *target_var);
    }
    
    info!(target: "time", "Starting book parser");
    
    // Fetch Labirint sitemap
    let resp = reqwest::get(LABIRINT_SITEMAP_URL)
        .await
        .context("Failed to fetch Labirint sitemap")?
        .text()
        .await
        .context("Failed to read Labirint sitemap body")?;
    
    let urlset: UrlSet = from_str(&resp)
        .context("Failed to parse Labirint sitemap XML")?;
    
    info!(target: "time", count = urlset.urls.len(), "Fetched Labirint sitemap URLs");

    // Prepare CSV writer
    let mut wtr = csv::Writer::from_path("books.csv")
        .context("Failed to create CSV file")?;
    wtr.write_record(BOOK_CSV_HEADERS)
        .context("Failed to write CSV headers")?;

    // Collect URLs from both sources
    let urls_labirint = urlset
        .urls
        .into_iter()
        .map(|u| u.loc)
        .filter(|u| u.contains("/books/"))
        .take(books_per_store);
    
    let urls_igraslov = {
        let mut books: Vec<String> = Vec::new();
        
        // Fetch from both sitemaps if threshold exceeded
        if books_per_store > IGRASLOV_SECOND_SITEMAP_THRESHOLD {
            let mut first_part = parse_sitemap_igraslov(IGRASLOV_SITEMAP_URLS[0])
                .await
                .context("Failed to parse first IgraSlov sitemap")?;
            let mut second_part = parse_sitemap_igraslov(IGRASLOV_SITEMAP_URLS[1])
                .await
                .context("Failed to parse second IgraSlov sitemap")?;
            
            books.append(&mut first_part);
            books.append(&mut second_part);
        } else {
            books = parse_sitemap_igraslov(IGRASLOV_SITEMAP_URLS[0])
                .await
                .context("Failed to parse IgraSlov sitemap")?;
        }
        
        books
    }
    .into_iter()
    .take(books_per_store);
    
    // Interleave URLs from both stores for balanced processing
    let urls = interleave(urls_igraslov, urls_labirint);

    // Process URLs concurrently
    let counter = Arc::new(AtomicU64::new(0));
    let total_urls = 2 * books_per_store;
    
    let books: Vec<_> = stream::iter(urls)
        .map(|url| {
            let counter = Arc::clone(&counter);
            async move {
                let result = if url.contains("labirint") {
                    parse_book_page(&LabirintParser, url).await
                } else if url.contains("igraslov") {
                    parse_book_page(&IgraSlov, url).await
                } else {
                    Err(anyhow!("Unknown bookstore domain in URL: {}", url))
                };
                
                let processed = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                println!("Processed: {}/{}", processed + 1, total_urls);
                
                result
            }
        })
        .buffer_unordered(concurrent_tasks)
        .collect()
        .await;
    
    // Write results to CSV
    let mut success_count = 0;
    let mut error_count = 0;
    
    for book in books.iter() {
        match book {
            Ok(book) => {
                info!("Successfully parsed book: {}", book.source);
                book.write_csv_record(&mut wtr)
                    .context("Failed to write CSV record")?;
                success_count += 1;
            }
            Err(e) => {
                warn!("Failed to parse book: {:#}", e);
                error_count += 1;
            }
        }
    }

    wtr.flush()
        .context("Failed to flush CSV writer")?;
    
    info!(target: "time", 
        "Parsing complete. Success: {}, Errors: {}", 
        success_count, 
        error_count
    );
    
    Ok(())
}

/// Parses a book page using the given parser.
///
/// This wrapper function adds tracing instrumentation.
#[tracing::instrument(skip(parser), fields(url=%url))]
async fn parse_book_page<T: BookParser>(
    parser: &T, 
    url: T::Url
) -> anyhow::Result<Book<T::Url>> {
    parser.parse_book(url).await
}

/// Interleaves elements from two iterators, alternating between them.
///
/// Once one iterator is exhausted, continues with the other.
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
