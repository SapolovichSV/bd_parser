use crate::parse_traits::{self, Author, BookParser, Isbn, Sites, Title};
use anyhow::anyhow;

pub struct LabirintParser;
impl BookParser for LabirintParser {
    const SITE: parse_traits::Sites = Sites::Labirint;
    type Url = String;

    type Context = scraper::Html;

    async fn fetch(&self, url: &Self::Url) -> anyhow::Result<Self::Context> {
        if !url.contains("books") {
            return Err(anyhow!("bad url").context(url.to_string()));
        }
        let page = reqwest::get(url).await?.text().await?;
        Ok(scraper::Html::parse_document(&page))
    }

    async fn parse_authors(
        &self,
        ctx: &Self::Context,
        url: &Self::Url,
    ) -> anyhow::Result<Vec<Author>> {
        let author_selector =
            scraper::Selector::parse("._left_u86in_12 > div:nth-child(1) > div:nth-child(2)")
                .map_err(|err| anyhow!("bad selector {err}").context(url.to_string()))?;

        Ok(ctx
            .select(&author_selector)
            .map(|node| Author::new(node.text().collect::<String>()))
            .collect())
    }

    async fn parse_isbn(&self, ctx: &scraper::Html, url: &Self::Url) -> anyhow::Result<Isbn> {
        let isbn_selector =
            scraper::Selector::parse("._right_u86in_12 > div:nth-child(2) > div:nth-child(2)")
                .map_err(|err| anyhow!("bad selector {err}").context(url.to_string()))?;

        match ctx.select(&isbn_selector).next_back() {
            Some(elem) => {
                let isbn_text: String = elem.text().collect::<String>().replace("\u{a0}", "");
                Isbn::new(isbn_text)
            }
            None => Err(anyhow!("can't find isbn on this page").context(url.to_string())),
        }
    }
    async fn parse_title(
        &self,
        ctx: &Self::Context,
        log_url: &Self::Url,
    ) -> anyhow::Result<parse_traits::Title> {
        let book_title_selector = scraper::Selector::parse("._h1_5o36c_18").map_err(|_err| {
            anyhow!("can't find title on this page").context(log_url.to_string())
        })?;
        Ok(Title::new(
            ctx.select(&book_title_selector)
                .map(|node| node.text().collect::<String>())
                .collect::<String>(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_HTML: &str = r#"
<!DOCTYPE html>
<html>
<body>
    <div class="_left_u86in_12">
        <div>
            <div>Автор:</div>
            <div>Лев Толстой</div>
        </div>
    </div>
    <div class="_right_u86in_12">
        <div>Placeholder</div>
        <div>
            <div>ISBN Label</div>
            <div>978-5-17-123456-7</div>
        </div>
    </div>
    <h1 class="_h1_5o36c_18">Война и мир</h1>
</body>
</html>
"#;

    const TEST_URL: &str = "https://www.labirint.ru/books/123456/";
    const EXPECTED_ISBN: &str = "978-5-17-123456-7";
    const EXPECTED_TITLE: &str = "Война и мир";
    const EXPECTED_AUTHOR: &str = "Лев Толстой";

    fn create_test_context() -> scraper::Html {
        scraper::Html::parse_document(TEST_HTML)
    }

    #[tokio::test]
    async fn test_parse_authors() {
        let parser = LabirintParser;
        let ctx = create_test_context();
        let url = TEST_URL.to_string();

        let result = parser.parse_authors(&ctx, &url).await;
        assert!(result.is_ok(), "parse_authors failed: {:?}", result.err());

        let authors = result.unwrap();
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].as_str(), EXPECTED_AUTHOR);
    }

    #[tokio::test]
    async fn test_parse_isbn() {
        let parser = LabirintParser;
        let ctx = create_test_context();
        let url = TEST_URL.to_string();

        let result = parser.parse_isbn(&ctx, &url).await;
        assert!(result.is_ok(), "parse_isbn failed: {:?}", result.err());

        let isbn = result.unwrap();
        assert_eq!(isbn.as_str(), EXPECTED_ISBN);
    }

    #[tokio::test]
    async fn test_parse_title() {
        let parser = LabirintParser;
        let ctx = create_test_context();
        let url = TEST_URL.to_string();

        let result = parser.parse_title(&ctx, &url).await;
        assert!(result.is_ok());

        let title = result.unwrap();
        assert_eq!(title.as_str(), EXPECTED_TITLE);
    }

    #[tokio::test]
    async fn test_parse_isbn_not_found() {
        let parser = LabirintParser;
        let html = scraper::Html::parse_document("<html><body></body></html>");
        let url = TEST_URL.to_string();

        let result = parser.parse_isbn(&html, &url).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_invalid_url() {
        let parser = LabirintParser;
        let invalid_url = "https://www.labirint.ru/invalid/".to_string();

        let result = parser.fetch(&invalid_url).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parse_book_integration() {
        let parser = LabirintParser;
        let url = "https://www.labirint.ru/books/801841/".to_string();

        let result = parser.parse_book(url).await;
        assert!(result.is_ok());

        let book = result.unwrap();
        assert!(!book.authors.is_empty());
        assert!(!book.isbn.as_str().is_empty());
        assert!(!book.title.as_str().is_empty());
    }
}
