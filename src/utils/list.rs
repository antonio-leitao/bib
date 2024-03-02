use anyhow::Result;
use std::io::{self, Stdout, Write};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

const WORDS: [&str; 20] = [
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
];

pub fn prompt_select() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().into_raw_mode().unwrap();

    // Move the cursor to the bottom of the previous output before starting
    //hide cursor
    write!(stdout, "{}", termion::cursor::Hide)?;
    let mut current_index = 0;
    draw_ui(&mut stdout, current_index)?;

    for c in stdin.keys() {
        match c.unwrap() {
            Key::Up | Key::Char('k') if current_index > 0 => {
                current_index -= 1;
                draw_ui(&mut stdout, current_index)?;
            }
            Key::Down | Key::Char('j') if current_index < WORDS.len() - 1 => {
                current_index += 1;
                draw_ui(&mut stdout, current_index)?;
            }
            Key::Char('\n') | Key::Char('q') | Key::Esc | Key::Ctrl('c') => break,
            _ => {}
        }
    }

    // Clean screen
    write!(
        stdout,
        "{}{}",
        termion::clear::AfterCursor,
        termion::cursor::Show
    )?;
    Ok(())
}

fn draw_ui(stdout: &mut RawTerminal<Stdout>, current_index: usize) -> Result<()> {
    // Move the cursor to the first line of the UI

    for (i, word) in WORDS.iter().enumerate() {
        let prefix = if i == current_index { "* " } else { "  " };
        writeln!(stdout, "{}{}\r", prefix, word)?;
    }

    write!(stdout, "{}", termion::cursor::Up(WORDS.len() as u16),)?;

    stdout.flush()?;
    Ok(())
}
