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

pub fn print_yeet(from: String, remote: Option<String>, into: String, number: usize) {
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

pub fn print_yank(from: String, remote: Option<String>, into: String, number: usize) {
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

