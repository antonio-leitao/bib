use anyhow::Result;

pub fn export(out: Option<String>) -> Result<()> {
    match out {
        Some(out) => println!("Printing current stack to: {},bib", out),
        None => println!("Printing current stack to stack.bib"),
    };
    Ok(())
}
