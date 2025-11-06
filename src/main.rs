//! Book parser library for scraping book information from various online stores.
//! 
//! Currently supports:
//! - Labirint.ru
//! - IgraSlov.store

use std::error::Error;

mod csv_save;
mod igraslov;
mod labirint;
mod parse_traits;
mod telemetry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("bd_parser - Book information parser");
    println!("Currently in development mode. Uncomment the main parsing logic to use.");
    Ok(())
}
