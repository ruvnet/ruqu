use ruqu_observer_world::{project_input, read_bounded};
use std::io::Write;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--trusted-root" {
        return Err("usage: ruqu-observer-world --trusted-root <independently trusted 64-character root> < report.json".into());
    }
    let bytes = read_bounded(std::io::stdin().lock())?;
    let projection = project_input(&bytes, &args[1])?;
    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
    serde_json::to_writer(&mut stdout, &projection)?;
    writeln!(stdout)?;
    stdout.flush()?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("observer-world: {error}");
        std::process::exit(1);
    }
}
