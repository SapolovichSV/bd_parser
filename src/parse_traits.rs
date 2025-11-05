use anyhow::Result;
use std::fmt::Display;

use reqwest::IntoUrl;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Isbn(String);

impl Isbn {
    pub fn new(s: String) -> Result<Self> {
        let cleaned = s.trim().replace(['-', ' '], "");
        if cleaned.len() >= 10 && cleaned.len() <= 13 && cleaned.chars().all(|c| c.is_ascii_digit())
        {
            Ok(Self(s))
        } else {
            anyhow::bail!("Invalid ISBN length or format: {}", cleaned.len())
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Isbn {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self> {
        Isbn::new(s)
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
}
impl Display for Sites {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Labirint => write!(f, "labirint"),
            Self::IgraSlov => write!(f, "igra_slov"),
        }
    }
}
#[derive(Debug)]
pub struct Book<T: IntoUrl + Into<String> + Display + Clone> {
    pub authors: Vec<Author>,
    pub isbn: Isbn,
    pub source: T,
    pub title: Title,
    pub site: Sites,
}
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
