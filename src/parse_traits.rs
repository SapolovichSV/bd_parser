//! Data structures and traits for parsing book information from websites.
//!
//! This module defines the core types (`Isbn`, `Author`, `Title`, `Book`) and the
//! `BookParser` trait that must be implemented by site-specific parsers.

use anyhow::{Result, anyhow};
use std::fmt::Display;
use tracing::{info, instrument};

use reqwest::IntoUrl;

/// International Standard Book Number (ISBN).
///
/// Can be either ISBN-10 or ISBN-13 format. The ISBN is stored in its original
/// format (with hyphens/spaces preserved) after validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Isbn(String);

impl Isbn {
    /// Creates a new ISBN after validating the format.
    ///
    /// # Errors
    ///
    /// Returns an error if the ISBN doesn't contain 10-13 digits.
    fn new(s: String) -> Result<Self> {
        let cleaned = s.trim().replace(['-', ' '], "");
        let digit_count = cleaned.chars().filter(|c| c.is_ascii_digit()).count();
        
        if (10..=13).contains(&digit_count) && cleaned.chars().all(|c| c.is_ascii_digit()) {
            Ok(Self(s))
        } else {
            anyhow::bail!(
                "Invalid ISBN format. Expected 10-13 digits, got {} digits in '{}'", 
                digit_count, 
                s
            )
        }
    }

    /// Returns the ISBN as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Parses raw ISBN text that may contain multiple ISBNs separated by commas or semicolons.
    ///
    /// Prefers ISBN-13 over ISBN-10 when multiple ISBNs are present.
    ///
    /// # Errors
    ///
    /// Returns an error if no valid ISBN is found in the input.
    #[instrument(ret)]
    fn parse(raw: String) -> anyhow::Result<String> {
        let tokens: Vec<&str> = raw
            .split([',', ';'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        
        if tokens.is_empty() {
            return Err(anyhow!("No ISBN detected in input"));
        }
        
        // Prefer ISBN-13 if available
        if let Some(isbn13) = tokens.iter().find(|t| Isbn::is_digit_13(t)) {
            info!(
                count = tokens.len(),
                "Multiple ISBNs found, preferring ISBN-13"
            );
            return Ok(isbn13.to_string());
        }
        
        // Otherwise use the last token (most likely to be the correct ISBN)
        tokens.last()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("Failed to extract ISBN from tokens"))
    }
    
    /// Checks if an ISBN string contains exactly 13 digits.
    fn is_digit_13(isbn: &str) -> bool {
        isbn.chars().filter(|c| c.is_ascii_digit()).count() == 13
    }
}

impl TryFrom<String> for Isbn {
    type Error = anyhow::Error;

    #[instrument]
    fn try_from(s: String) -> Result<Self> {
        Isbn::new(Isbn::parse(s)?)
    }
}

impl Display for Isbn {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Book author name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Author(pub String);

impl Author {
    /// Creates a new author, trimming whitespace.
    pub fn new(s: String) -> Self {
        Author(s.trim().to_string())
    }

    /// Returns the author name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Author {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self> {
        Ok(Author::new(s))
    }
}

impl Display for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Book title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Title(pub String);

impl Title {
    /// Creates a new title, trimming whitespace.
    pub fn new(s: String) -> Self {
        Title(s.trim().to_string())
    }

    /// Returns the title as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Title {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self> {
        Ok(Title::new(s))
    }
}

impl Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Enumeration of supported book store sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sites {
    /// Labirint bookstore (labirint.ru)
    Labirint,
    /// IgraSlov bookstore (igraslov.store)
    IgraSlov,
}

impl Display for Sites {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Labirint => write!(f, "labirint"),
            Self::IgraSlov => write!(f, "igra_slov"),
        }
    }
}

/// Represents a book with its metadata and source URL.
#[derive(Debug)]
pub struct Book<T: IntoUrl + Into<String> + Display + Clone> {
    /// List of book authors
    pub authors: Vec<Author>,
    /// International Standard Book Number
    pub isbn: Isbn,
    /// Source URL where the book information was fetched
    pub source: T,
    /// Book title
    pub title: Title,
    /// The bookstore site this book was parsed from
    pub site: Sites,
}

/// Trait for parsing book information from website HTML.
///
/// Implementors must provide site-specific parsing logic for extracting
/// book metadata (authors, ISBN, title) from HTML pages.
pub trait BookParser {
    /// The bookstore site this parser is for.
    const SITE: Sites;

    /// URL type for this parser (typically String).
    type Url: IntoUrl + Into<String> + Display + Clone + Send + Sync + 'static;

    /// Context type holding parsed HTML or intermediate data.
    type Context: Send;

    /// Fetches and parses the HTML page at the given URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the URL is invalid.
    async fn fetch(&self, url: &Self::Url) -> Result<Self::Context>;
    
    /// Parses author information from the page context.
    ///
    /// # Errors
    ///
    /// Returns an error if authors cannot be extracted.
    async fn parse_authors(&self, ctx: &Self::Context, log_url: &Self::Url) -> Result<Vec<Author>>;
    
    /// Parses ISBN from the page context.
    ///
    /// # Errors
    ///
    /// Returns an error if ISBN is not found or invalid.
    async fn parse_isbn(&self, ctx: &Self::Context, log_url: &Self::Url) -> Result<Isbn>;
    
    /// Parses book title from the page context.
    ///
    /// # Errors
    ///
    /// Returns an error if title cannot be extracted.
    async fn parse_title(&self, ctx: &Self::Context, log_url: &Self::Url) -> Result<Title>;

    /// Parses a complete book from a URL by fetching and extracting all fields.
    ///
    /// # Errors
    ///
    /// Returns an error if fetching fails or any required field cannot be parsed.
    async fn parse_book(&self, url: Self::Url) -> Result<Book<Self::Url>> {
        let ctx = self.fetch(&url).await?;
        let authors = self.parse_authors(&ctx, &url).await?;
        let title = self.parse_title(&ctx, &url).await?;
        let isbn = self.parse_isbn(&ctx, &url).await?;
        Ok(Book {
            authors,
            isbn,
            source: url,
            title,
            site: Self::SITE,
        })
    }
}
