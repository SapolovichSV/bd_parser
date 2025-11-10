use std::{sync::OnceLock, time::Duration};

use anyhow::anyhow;
use tracing::{instrument, warn};

use crate::parse_traits::{Author, BookParser, Description, Isbn, Sites, Title};

static AUTHOR_SEL_STR: &str = ".book-page__card-author-link";
static ISBN_SEL_STR: &str = "span.copy__val";
static TITLE_SEL_STR: &str = ".book-page__card-title";
static DESCR_SEL_STR: &str =
    "div.spoiler__text.t.t_last-p-no-offset.book-page__card-description-text p";

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static AUTHOR_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static ISBN_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static TITLE_SEL: OnceLock<scraper::Selector> = OnceLock::new();
static DESCR_SEL: OnceLock<scraper::Selector> = OnceLock::new();
pub struct EksmoParser;
impl BookParser for EksmoParser {
    const SITE: crate::parse_traits::Sites = Sites::Eksmo;

    type Url = String;

    type Context = scraper::Html;

    #[instrument(skip(self, url))]
    async fn fetch(&self, url: &Self::Url) -> anyhow::Result<Self::Context> {
        let client = CLIENT.get_or_init(|| {
            reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(15))
                .pool_max_idle_per_host(4)
                .tcp_keepalive(Some(Duration::from_secs(30)))
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .expect("http client")
        });
        match client.get(url).send().await {
            Ok(response) if !response.status().is_success() => {
                warn!(
                    "bad status code probably rate limit code: {}",
                    response.status()
                );
                return Err(anyhow!("response status is not success"));
            }
            Ok(response) => {
                let resp = response.text().await?;
                Ok(scraper::Html::parse_document(&resp))
            }
            Err(e) => return Err(e.into()),
        }
    }
    #[instrument(skip(self, ctx, _log_url))]
    async fn parse_authors(
        &self,
        ctx: &Self::Context,
        _log_url: &Self::Url,
    ) -> anyhow::Result<Vec<crate::parse_traits::Author>> {
        let author_selector = AUTHOR_SEL
            .get_or_init(|| scraper::Selector::parse(AUTHOR_SEL_STR).expect("author selector"));

        Ok(ctx
            .select(author_selector)
            .map(|node| Author::new(node.text().collect::<String>()))
            .collect())
    }

    #[instrument(skip(self, ctx, _log_url))]
    async fn parse_isbn(
        &self,
        ctx: &Self::Context,
        _log_url: &Self::Url,
    ) -> anyhow::Result<crate::parse_traits::Isbn> {
        let isbn_selector =
            ISBN_SEL.get_or_init(|| scraper::Selector::parse(ISBN_SEL_STR).expect("isbn selector"));

        match ctx.select(isbn_selector).next_back() {
            Some(elem) => {
                let raw: String = elem.text().collect::<String>().replace("\u{a0}", "");
                match Isbn::try_from(raw) {
                    Ok(isbn) => Ok(isbn),
                    Err(e) => {
                        warn!("can't parse isbn:{e}");
                        Err(anyhow!("can't parse isbn"))
                    }
                }
            }
            None => {
                warn!(target: "time","ISBN not found on page{_log_url}");
                Err(anyhow!("can't find isbn on this page"))
            }
        }
    }

    #[instrument(skip(self, ctx, _log_url))]
    async fn parse_title(
        &self,
        ctx: &Self::Context,
        _log_url: &Self::Url,
    ) -> anyhow::Result<crate::parse_traits::Title> {
        let book_title_selector = TITLE_SEL
            .get_or_init(|| scraper::Selector::parse(TITLE_SEL_STR).expect("title selector"));
        let title = {
            ctx.select(book_title_selector)
                .map(|node| node.text().collect::<String>())
                .collect::<String>()
        };
        Ok(Title::new(title))
    }

    #[instrument(skip(self, ctx))]
    async fn parse_description(
        &self,
        ctx: &Self::Context,
    ) -> anyhow::Result<crate::parse_traits::Description> {
        let book_descr_sel = DESCR_SEL
            .get_or_init(|| scraper::Selector::parse(DESCR_SEL_STR).expect("descr selector"));
        let descr = ctx
            .select(book_descr_sel)
            .map(|p| p.text().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        Ok(Description::new(descr))
    }
    #[instrument(skip(self),fields(url=&url))]
    async fn parse_book(
        &self,
        url: Self::Url,
    ) -> anyhow::Result<crate::parse_traits::Book<Self::Url>> {
        let ctx = self.fetch(&url).await?;
        let authors = self.parse_authors(&ctx, &url).await?;
        let title = self.parse_title(&ctx, &url).await?;
        let isbn = self.parse_isbn(&ctx, &url).await?;
        let description = self.parse_description(&ctx).await?;
        Ok(crate::parse_traits::Book {
            authors,
            isbn,
            source: url,
            title,
            site: Self::SITE,
            description,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED_AUTHOR: &str = "Андрей Самарин";
    const EXPECTED_ISBN: &str = "978-5-04-156838-2";
    const EXPECTED_TITLE: &str =
        "Структура таланта. От иллюзий к реальности: как стать настоящим художником";

    const EXPECTED_DESCRIPTION: &str = r###"Книга, которая поможет взглянуть на феномен «таланта» без розовых очков.

Андрей Самарин, российский художественный деятель, преподаватель и основатель студии рисования, разбирает мифы о врожденных способностях и показывает, что за успехом всегда стоят конкретные навыки, практика и систематический подход. С опорой на психологические исследования, истории из разных сфер и практические рекомендации книга объясняет, как формируются способности, как их развивать и почему одни добиваются результата, а другие так и остаются пребывать в иллюзиях.

Внутри:

- доступное объяснение, как работает «талант» с точки зрения науки;

- реальные истории и кейсы, подтверждающие выводы автора;

- стратегии, которые помогут развивать собственные навыки и перестать ждать чуда;

- советы по созданию условий для роста в учебе, работе и творчестве.

Эта книга — находка для всех, кто хочет развить свой потенциал, перестать верить в явление урожденного гения и понять, как устроен истинный путь к мастерству. Подходит студентам, педагогам, руководителям и всем, кто интересуется развитием человека.

Что такое талант и как его обрести?

В книге "Структура таланта" художник Андрей Самарин исследует внутренний мир творцов, особенности их мышления и подхода к искусству. Автор раскрывает, как сочетание уникального восприятия, дисциплины, смелости и внутренней честности формирует путь к успеху. Вместе с ним вы разберете, что такое талант, с точки зрения когнитивного навыка. Вы разоблачите мифы и иллюзии, связанные с творческими профессиями. В практической части на примере рисования автор расскажет, какой подход в обучении по-настоящему эффективен и какие существуют неочевидные, но ключевые нюансы, о которых не говорят в традиционных программах. Вы поговорите об искусстве, мастерстве и творчестве, их месте на рынке в условиях инклюзивного тренда, а также о влиянии ИИ на развитие современного художника и других факторах, определяющих его новую роль."###;
    fn get_context() -> scraper::Html {
        let context = include_str!("../page_examples/eksmo.html");
        scraper::Html::parse_document(context)
    }

    #[tokio::test]
    async fn parse_authors_ok() {
        let parser = EksmoParser;
        let ctx = get_context();
        let url = "https://eksmo.ru/book/example".to_string();
        let authors = parser
            .parse_authors(&ctx, &url)
            .await
            .expect("authors parsed");
        assert!(!authors.is_empty());
        assert_eq!(authors[0].as_str(), EXPECTED_AUTHOR);
    }

    #[tokio::test]
    async fn parse_isbn_ok() {
        let parser = EksmoParser;
        let ctx = get_context();
        let url = "https://eksmo.ru/book/example".to_string();
        let isbn = parser.parse_isbn(&ctx, &url).await.expect("isbn parsed");
        assert_eq!(isbn.as_str(), EXPECTED_ISBN);
        let digits = isbn.as_str().chars().filter(|c| c.is_ascii_digit()).count();
        assert_eq!(digits, 13);
    }

    #[tokio::test]
    async fn parse_title_ok() {
        let parser = EksmoParser;
        let ctx = get_context();
        let url = "https://eksmo.ru/book/example".to_string();
        let title = parser.parse_title(&ctx, &url).await.expect("title parsed");
        assert_eq!(title.as_str(), EXPECTED_TITLE);
    }
    fn normalize_text(s: &str) -> String {
        s.replace("\r", "") // убрать \r, если есть
            .lines() // пройтись по строкам
            .map(|l| l.trim()) // обрезать пробелы по краям
            .filter(|l| !l.is_empty()) // убрать пустые строки
            .collect::<Vec<_>>()
            .join("\n") // собрать с одним переносом
    }

    #[tokio::test]
    async fn parse_description() {
        let parser = EksmoParser;
        let ctx = get_context();
        let descr = parser.parse_description(&ctx).await.expect("should");
        let descr = normalize_text(descr.as_str());
        let expected = normalize_text(EXPECTED_DESCRIPTION);
        assert_eq!(descr, expected);
    }
    #[tokio::test]
    async fn parse_isbn_not_found() {
        let parser = EksmoParser;
        let empty_ctx = scraper::Html::parse_document("<html><body></body></html>");
        let url = "https://eksmo.ru/book/example".to_string();
        let res = parser.parse_isbn(&empty_ctx, &url).await;
        assert!(res.is_err());
    }
}
