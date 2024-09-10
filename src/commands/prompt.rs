use crate::base::{Item, Paper};
use crate::embedding::Point;
use crate::{
    base::load_papers,
    embedding::{encode, k_nearest, load_vectors},
    utils::io::read_config_file,
};
use anyhow::Result;
use std::cmp;
use std::collections::BTreeMap;
use std::io::{self, Stdout, Write};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

fn filter_by_stack(papers: &BTreeMap<String, Paper>) -> Result<Vec<String>> {
    let config = read_config_file()?;
    let indicies: Vec<String> = match config.current_stack() {
        Some(current) => papers
            .iter()
            .filter(|(_key, paper)| paper.stack.iter().any(|stack| *stack == current))
            .map(|(key, _paper)| key.clone())
            .collect(),

        None => papers.keys().map(|key| key.clone()).collect(),
    };
    Ok(indicies)
}

fn filter_by_query(
    query: String,
    points: &BTreeMap<String, Point>,
    indicies: &Vec<String>,
    k: usize,
) -> Result<Vec<String>> {
    let query = encode(&query)?;
    Ok(k_nearest(&query, points, indicies, k))
}

fn prompt_select(papers: &[Paper]) -> Result<Option<usize>> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().into_raw_mode().unwrap();
    let (width, _) = termion::terminal_size().unwrap();
    let mut selected_index: Option<usize> = None;
    // Move the cursor to the bottom of the previous output before starting
    //hide cursor
    write!(stdout, "{}", termion::cursor::Hide)?;
    let mut current_index = 0;
    draw_ui(&mut stdout, current_index, papers, width)?;

    for c in stdin.keys() {
        match c.unwrap() {
            Key::Up | Key::Char('k') if current_index > 0 => {
                current_index -= 1;
                draw_ui(&mut stdout, current_index, papers, width)?;
            }
            Key::Down | Key::Char('j') if current_index < papers.len() - 1 => {
                current_index += 1;
                draw_ui(&mut stdout, current_index, papers, width)?;
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

fn draw_ui(
    stdout: &mut RawTerminal<Stdout>,
    current_index: usize,
    items: &[Paper],
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

pub fn list(max: Option<usize>) -> Result<()> {
    //Loading bigliography
    let (width, height) = termion::terminal_size()?;
    let papers = load_papers()?;
    let n_refs = papers.len();
    let indicies = filter_by_stack(&papers)?;

    // Determine the maximum number of entries to display
    let max_entries = match max {
        Some(m) => cmp::min(m, n_refs),
        None => cmp::min(height as usize - 4, n_refs),
    };

    // Iterate over the specified number of elements
    indicies
        .iter()
        .filter_map(|key| papers.get(key).cloned())
        .rev()
        .take(max_entries)
        .for_each(|paper| println!("{}", paper.display(width)));

    // Print a message if there are more references not being displayed
    if n_refs > max_entries {
        println!(
            "\t ----- hiding other {} references -----",
            n_refs - max_entries
        );
    }
    Ok(())
}

pub fn open(query: String) -> Result<()> {
    let papers = load_papers()?;
    let points = load_vectors()?;
    let (width, height) = termion::terminal_size()?;
    let mut indicies = filter_by_stack(&papers)?;
    if query.len() > 0 {
        indicies = filter_by_query(query, &points, &indicies, height as usize - 10)?;
    };
    let items: Vec<Paper> = indicies
        .iter()
        .filter_map(|key| papers.get(key).cloned())
        .collect();
    match prompt_select(&items)? {
        Some(index) => println!("{}", items[index].display(width)),
        None => (),
    };
    Ok(())
}
pub fn yank(query: String) {
    println!("Copying bibfile with query: {:?}", query)
}
