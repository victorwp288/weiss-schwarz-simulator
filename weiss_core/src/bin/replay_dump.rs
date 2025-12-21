use std::env;
use std::path::PathBuf;
use weiss_core::replay::read_replay_file;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: replay_dump <replay_file>");
        std::process::exit(1);
    }
    let path = PathBuf::from(&args[1]);
    let data = read_replay_file(&path)?;
    let json = serde_json::to_string(&data)?;
    println!("{}", json);
    Ok(())
}
