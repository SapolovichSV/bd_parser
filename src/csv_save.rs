use std::fmt::Display;

use reqwest::IntoUrl;

use crate::parse_traits::Book;

pub static BOOK_CSV_HEADERS: &[&str] = &["site", "source", "isbn", "title", "authors"];

pub trait CsvSave {
    fn csv_headers() -> &'static [&'static str];
    fn write_csv_record<W: std::io::Write>(&self, wtr: &mut csv::Writer<W>) -> csv::Result<()>;
}

impl<T> CsvSave for Book<T>
where
    T: IntoUrl + Into<String> + Display + Clone,
{
    fn csv_headers() -> &'static [&'static str] {
        BOOK_CSV_HEADERS
    }

    fn write_csv_record<W: std::io::Write>(&self, wtr: &mut csv::Writer<W>) -> csv::Result<()> {
        let authors_joined = self
            .authors
            .iter()
            .map(|a| a.as_str())
            .collect::<Vec<_>>()
            .join("; ");

        wtr.write_record(&[
            format!("{}", self.site),
            self.source.to_string(),
            self.isbn.to_string(),
            self.title.to_string(),
            authors_joined,
        ])
    }
}
