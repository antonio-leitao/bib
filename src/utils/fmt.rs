use crate::base::Paper;
use termion::color;

pub trait Clean {
    fn clean(&self) -> Self;
}

impl Clean for String {
    fn clean(&self) -> Self {
        let cleaned_str = self
            .chars() // Get iterator over characters
            .filter(|&c| !c.is_control()) // Filter out control characters (e.g., newline, tab)
            .collect(); // Collect the characters back into a String
        cleaned_str
    }
}

pub fn init() {
    println!(
        "Initiated 'bib' with {}base{} stack",
        color::Fg(color::LightGreen),
        color::Fg(color::Reset)
    );
}

pub fn yeet(from: String, remote: Option<String>, into: String, number: usize) {
    if number < 1 {
        println!("Nothing to yeet");
        return;
    }
    let into = match remote {
        Some(remote) => format!("{}/{}", remote, into),
        None => into,
    };
    println!("Pushed {} references to {}", number, into);
    print!("  {} -> {} | {}", from, into, color::Fg(color::LightGreen));
    for _ in 0..number {
        print!("+");
    }
    println!("{}", color::Fg(color::Reset));
}

pub fn yank(from: String, remote: Option<String>, into: String, number: usize) {
    if number < 1 {
        println!("Nothing to yank");
        return;
    }
    let from = match remote {
        Some(remote) => format!("{}/{}", remote, from),
        None => from,
    };
    println!("Pulled {} references from {}", number, from);
    print!("  {} <- {} | {}", into, from, color::Fg(color::LightGreen));
    for _ in 0..number {
        print!("+");
    }
    println!("{}", color::Fg(color::Reset));
}

pub fn merge(from: String, into: String, number: usize) {
    if number < 1 {
        println!("Stack is up to date");
        return;
    }
    println!("Merged {} references from {}", number, from);
    print!("  {} <- {} | {}", into, from, color::Fg(color::LightGreen));
    for _ in 0..number {
        print!("+");
    }
    println!("{}", color::Fg(color::Reset));
}

pub fn switch(from: String, into: String) {
    println!("Moving to {} stack", into);
    println!(
        "  {} -> {}{}{}",
        from,
        color::Fg(color::Green),
        into,
        color::Fg(color::Reset)
    );
}

pub fn new(stack: String) {
    println!(
        "{}+{} Created new {} stack",
        color::Fg(color::LightGreen),
        color::Fg(color::Reset),
        stack,
    );
}

pub fn rename(from: String, into: String) {
    println!("Renamed to {} stack", into.clone());
    println!(
        "{} -> {}{}{}",
        from,
        color::Fg(color::Green),
        into,
        color::Fg(color::Reset),
    );
}

pub fn export(from: String, into: String) {
    println!("Exported {} stack", from.clone());
    println!(
        "{}{}{} -> {}.bib",
        color::Fg(color::Green),
        from,
        color::Fg(color::Reset),
        into,
    );
}

pub fn add(into: String, paper: Paper) {
    println!("Added reference");
    println!(
        "  {} <- {}+{} | {}",
        into,
        color::Fg(color::LightGreen),
        color::Fg(color::Reset),
        paper.title
    );
}

pub fn delete(stack: String) {
    println!(
        "{}x{} Deleted {} stack",
        color::Fg(color::Red),
        color::Fg(color::Reset),
        stack
    );
}
pub fn erro(err: String) {
    println!(
        "{}error{}: {}",
        color::Fg(color::Red),
        color::Fg(color::Reset),
        err
    );
}
