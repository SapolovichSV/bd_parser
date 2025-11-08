//! CSV serialization support for book data.

use std::fmt::Display;

use reqwest::IntoUrl;

use crate::parse_traits::Book;

/// CSV column headers for book export.
pub static BOOK_CSV_HEADERS: &[&str] = &["site", "source", "isbn", "title", "authors"];

/// Trait for types that can be saved to CSV format.
pub trait CsvSave {
    /// Writes a record to the CSV writer.
    ///
    /// # Errors
    ///
    /// Returns a CSV error if writing fails.
    fn write_csv_record<W: std::io::Write>(&self, wtr: &mut csv::Writer<W>) -> csv::Result<()>;
}

impl<T> CsvSave for Book<T>
where
    T: IntoUrl + Into<String> + Display + Clone,
{
    fn write_csv_record<W: std::io::Write>(&self, wtr: &mut csv::Writer<W>) -> csv::Result<()> {
        let authors_joined = self
            .authors
            .iter()
            .map(|a| a.as_str())
            .collect::<Vec<_>>()
            .join("; ");

        wtr.write_record(&[
            self.site.to_string(),
            self.source.to_string(),
            self.isbn.to_string(),
            self.title.to_string(),
            authors_joined,
        ])
    }
}
