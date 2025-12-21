use std::env;
use std::fs;
use std::path::PathBuf;

use weiss_core::db::{CardDb, CardStatic};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: carddb_pack <input_json> <output_wsdb>");
        std::process::exit(1);
    }
    let input = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let bytes = fs::read(&input)?;

    let db = match serde_json::from_slice::<CardDb>(&bytes) {
        Ok(db) => CardDb::new(db.cards)?,
        Err(_) => {
            let cards: Vec<CardStatic> = serde_json::from_slice(&bytes)?;
            CardDb::new(cards)?
        }
    };

    let out = db.to_bytes_with_header()?;
    fs::write(output, out)?;
    Ok(())
}
