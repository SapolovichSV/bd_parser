use anyhow::{Context, Result, anyhow};
use std::{fmt::Display, str::FromStr};
use tracing::{info, instrument};

use reqwest::IntoUrl;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Isbn(String);

impl Isbn {
    fn new(s: String) -> Result<Self> {
        let cleaned = s.trim().replace(['-', ' '], "");
        if cleaned.len() >= 10 && cleaned.len() <= 13 && cleaned.chars().all(|c| c.is_ascii_digit())
        {
            Ok(Self(s))
        } else {
            anyhow::bail!("Invalid ISBN:{} length or format: {}", s, cleaned.len())
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
    #[instrument(ret)]
    fn parse(raw: String) -> anyhow::Result<String> {
        let tokens: Vec<&str> = raw
            .split([',', ';'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if tokens.is_empty() {
            return Err(anyhow!("no isbn detected"));
        }
        if let Some(isbn13) = tokens.iter().find(|t| Isbn::is_digit_13(t)) {
            info!(
                count = tokens.len(),
                "multiplie ISBNs found, preferring ISBN-13"
            );
            return Ok(isbn13.to_string());
        }
        Ok(tokens.last().unwrap().to_string())
    }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sites {
    Labirint,
    IgraSlov,
    Eksmo,
}
impl Display for Sites {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Labirint => write!(f, "labirint"),
            Self::IgraSlov => write!(f, "igra_slov"),
            Self::Eksmo => write!(f, "eksmo"),
        }
    }
}
#[derive(Debug)]
pub struct Description(String);
impl Description {
    pub fn new(s: String) -> Self {
        Self { 0: s }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Debug)]
pub struct Price(u128);

impl From<u128> for Price {
    fn from(value: u128) -> Self {
        Self(value)
    }
}
impl From<Price> for u128 {
    fn from(value: Price) -> Self {
        value.0
    }
}
impl TryFrom<String> for Price {
    type Error = anyhow::Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        if value.chars().all(|char| char.is_ascii_digit()) {
            return Err(anyhow!("forbidded symbol in {value}"));
        }
        let res: u128 = value
            .parse()
            .map_err(|e| anyhow!("can't parse as u128 value:{value} error: {e}"))?;
        Ok(Self(res))
    }
}
impl Price {
    pub fn new(s: String) -> Self {
        let num = s.parse().unwrap();
        Self(num)
    }
}
impl FromStr for Price {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let num: u128 = match s.parse() {
            Ok(num) => num,
            Err(e) => return Err(anyhow!(e)),
        };
        Ok(Self(num))
    }
}

impl Display for Price {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub struct Book<T: IntoUrl + Into<String> + Display + Clone> {
    pub authors: Vec<Author>,
    pub isbn: Isbn,
    pub source: T,
    pub title: Title,
    pub site: Sites,
    pub description: Description,
    pub price: Price,
}
pub trait BookParser {
    const SITE: Sites;

    type Url: IntoUrl + Into<String> + Display + Clone + Send + Sync + 'static;

    type Context: Send;

    async fn fetch(&self, url: &Self::Url) -> Result<Self::Context>;
    async fn parse_authors(&self, ctx: &Self::Context, log_url: &Self::Url) -> Result<Vec<Author>>;
    async fn parse_isbn(&self, ctx: &Self::Context, log_url: &Self::Url) -> Result<Isbn>;
    async fn parse_title(&self, ctx: &Self::Context, log_url: &Self::Url) -> Result<Title>;
    async fn parse_description(&self, ctx: &Self::Context) -> Result<Description>;
    async fn parse_price(&self, ctx: &Self::Context) -> Result<Price>;

    #[instrument(skip(self),fields(url=%url))]
    async fn parse_book(&self, url: Self::Url) -> Result<Book<Self::Url>> {
        info!(target: "time","start processing");
        let ctx = self.fetch(&url).await?;
        let authors = self
            .parse_authors(&ctx, &url)
            .await
            .with_context(|| format!("parse_authors failed: {}", url))?;
        let title = self
            .parse_title(&ctx, &url)
            .await
            .with_context(|| format!("parse_title failed: {}", url))?;
        let isbn = self
            .parse_isbn(&ctx, &url)
            .await
            .with_context(|| format!("parse_isbn failed: {}", url))?;
        let description = self
            .parse_description(&ctx)
            .await
            .with_context(|| format!("parse_description failed: {}", url))?;
        let price = self
            .parse_price(&ctx)
            .await
            .with_context(|| format!("parce_price failed: {}", url))?;
        info!(target: "time","end processing");
        Ok(Book {
            authors,
            isbn,
            source: url,
            title,
            site: Self::SITE,
            description,
            price,
        })
    }
}
