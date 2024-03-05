use crate::utils::ui::Item;
use anyhow::Result;
use std::io::{self, Stdout, Write};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

pub fn prompt_select<T: Item + Clone>(items: &[T]) -> Result<Option<usize>> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().into_raw_mode().unwrap();
    let (width, _) = termion::terminal_size().unwrap();
    let mut selected_index: Option<usize> = None;
    // Move the cursor to the bottom of the previous output before starting
    //hide cursor
    write!(stdout, "{}", termion::cursor::Hide)?;
    let mut current_index = 0;
    draw_ui(&mut stdout, current_index, items, width)?;

    for c in stdin.keys() {
        match c.unwrap() {
            Key::Up | Key::Char('k') if current_index > 0 => {
                current_index -= 1;
                draw_ui(&mut stdout, current_index, items, width)?;
            }
            Key::Down | Key::Char('j') if current_index < items.len() - 1 => {
                current_index += 1;
                draw_ui(&mut stdout, current_index, items, width)?;
            }
            Key::Char('\n') => {
                selected_index = Some(current_index);
                break;
            }
            Key::Char('q') | Key::Esc | Key::Ctrl('c') => {
                selected_index = None;
                break;
            }
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
    Ok(selected_index)
}

fn draw_ui<T: Item + Clone>(
    stdout: &mut RawTerminal<Stdout>,
    current_index: usize,
    items: &[T],
    width: u16,
) -> Result<()> {
    // Move the cursor to the first line of the UI
    for (i, word) in items.iter().enumerate() {
        let prefix = if i == current_index { "* " } else { "  " };
        writeln!(stdout, "{}{}\r", prefix, word.display(width - 2))?;
    }

    write!(stdout, "{}", termion::cursor::Up(items.len() as u16))?;

    stdout.flush()?;
    Ok(())
}
