//! Core traits and types for book parsing.
//!
//! This module defines the fundamental types used across all parsers:
//! - `Book`: The main book data structure
//! - `BookParser`: Trait that all site-specific parsers must implement
//! - `Isbn`, `Author`, `Title`: Type-safe wrappers for book metadata
//! - `Sites`: Enum representing supported book stores

use anyhow::{anyhow, Result};
use reqwest::IntoUrl;
use std::fmt::Display;

/// A validated ISBN number (International Standard Book Number).
/// 
/// Supports both ISBN-10 and ISBN-13 formats with or without dashes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Isbn(String);

impl Isbn {
    /// Creates a new ISBN after validation.
    /// 
    /// # Arguments
    /// * `s` - ISBN string (may contain dashes and spaces)
    /// 
    /// # Returns
    /// Ok(Isbn) if the string contains exactly 10 or 13 digits
    fn new(s: String) -> Result<Self> {
        let cleaned = s.trim().replace(['-', ' '], "");
        let digit_count = cleaned.chars().filter(|c| c.is_ascii_digit()).count();
        
        if (digit_count == 10 || digit_count == 13) && cleaned.chars().all(|c| c.is_ascii_digit()) {
            Ok(Self(s))
        } else {
            anyhow::bail!("Invalid ISBN: expected 10 or 13 digits, found {}", digit_count)
        }
    }

    /// Returns the ISBN as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Parses ISBN from raw text that may contain multiple ISBNs separated by commas or semicolons.
    /// Prefers ISBN-13 over ISBN-10 when multiple ISBNs are present.
    fn parse(raw: String) -> Result<String> {
        let tokens: Vec<&str> = raw
            .split([',', ';'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        
        if tokens.is_empty() {
            return Err(anyhow!("No ISBN detected"));
        }
        
        // Prefer ISBN-13 over ISBN-10
        if let Some(isbn13) = tokens.iter().find(|t| Isbn::is_digit_13(t)) {
            return Ok(isbn13.to_string());
        }
        
        // Fall back to the last token (we know tokens is not empty due to earlier check)
        Ok(tokens.last().expect("tokens cannot be empty").to_string())
    }
    
    /// Checks if a string contains exactly 13 digits (for ISBN-13 detection).
    fn is_digit_13(isbn: &str) -> bool {
        isbn.chars().filter(|c| c.is_ascii_digit()).count() == 13
    }
}

impl TryFrom<String> for Isbn {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self> {
        Isbn::new(Isbn::parse(s)?)
    }
}

impl Display for Isbn {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A book author's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Author(pub String);

impl Author {
    pub fn new(s: String) -> Self {
        Author(s.trim().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Author {
    fn from(s: String) -> Self {
        Author::new(s)
    }
}

impl Display for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A book title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Title(pub String);

impl Title {
    pub fn new(s: String) -> Self {
        Title(s.trim().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Title {
    fn from(s: String) -> Self {
        Title::new(s)
    }
}

impl Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Supported book store websites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sites {
    Labirint,
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

/// A book with metadata scraped from a website.
#[derive(Debug)]
pub struct Book<T: IntoUrl + Into<String> + Display + Clone> {
    pub authors: Vec<Author>,
    pub isbn: Isbn,
    pub source: T,
    pub title: Title,
    pub site: Sites,
}

/// Trait for implementing site-specific book parsers.
/// 
/// Each website requires a different parsing strategy due to varying HTML structures.
/// Implementors must define how to fetch and parse book information from their specific site.
pub trait BookParser {
    const SITE: Sites;

    type Url: IntoUrl + Into<String> + Display + Clone + Send + Sync + 'static;

    type Context: Send;

    async fn fetch(&self, url: &Self::Url) -> Result<Self::Context>;
    async fn parse_authors(&self, ctx: &Self::Context, log_url: &Self::Url) -> Result<Vec<Author>>;
    async fn parse_isbn(&self, ctx: &Self::Context, log_url: &Self::Url) -> Result<Isbn>;
    async fn parse_title(&self, ctx: &Self::Context, log_url: &Self::Url) -> Result<Title>;

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
