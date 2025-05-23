use crate::base::{save_papers, Paper};
use crate::embedding::Point;
use crate::stacks::Stack;
use crate::{
    base::load_papers,
    embedding::{k_nearest, load_vectors},
    utils::io::read_config_file,
};
use crate::{blog, utils};
use anyhow::{anyhow, Result};
use copypasta::{ClipboardContext, ClipboardProvider};
use indexmap::IndexMap;
use std::cmp;
use std::collections::BTreeMap;
use std::io::{self, Stdout, Write};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

fn pull_up(map: &mut IndexMap<String, Paper>, key: &str) {
    if let Some(removed_paper) = map.shift_remove(key) {
        map.shift_insert(0, key.to_string(), removed_paper);
    }
}

fn filter_by_stack(papers: &IndexMap<String, Paper>) -> Result<Vec<String>> {
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
    let query = utils::ai::query_embedding_sync(&query)?;
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
    // State for toggling display between title and comment
    // false = show title, true = show comment (if available)
    let mut states: Vec<bool> = vec![false; papers.len()];
    draw_ui(&mut stdout, current_index, papers, width, &states)?;

    for c in stdin.keys() {
        match c.unwrap() {
            Key::Up | Key::Char('k') if current_index > 0 => {
                current_index -= 1;
                draw_ui(&mut stdout, current_index, papers, width, &states)?;
            }
            Key::Down | Key::Char('j') if current_index < papers.len() - 1 => {
                current_index += 1;
                draw_ui(&mut stdout, current_index, papers, width, &states)?;
            }
            Key::Char('\t') => {
                states[current_index] = !states[current_index];
                draw_ui(&mut stdout, current_index, papers, width, &states)?;
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
    item_display_states: &[bool],
) -> Result<()> {
    // Move the cursor to the first line of the UI
    for (i, word) in items.iter().enumerate() {
        let prefix = if i == current_index { "* " } else { "  " };
        write!(
            stdout,
            "{}{}{}",
            termion::clear::CurrentLine,
            prefix,
            word.display(width - 2, item_display_states[i])
        )?;
        writeln!(stdout, "\r")?;
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
        .take(max_entries)
        .for_each(|paper| println!("{}", paper.display(width, false)));

    // Print a message if there are more references not being displayed
    if n_refs > max_entries {
        println!(
            "\t ----- hiding other {} references -----",
            n_refs - max_entries
        );
    }
    Ok(())
}

fn select(query: String, papers: &IndexMap<String, Paper>) -> Result<Option<Paper>> {
    let points = load_vectors()?;
    let (_width, height) = termion::terminal_size()?;
    let mut indicies = filter_by_stack(papers)?;
    if query.len() > 0 {
        indicies = filter_by_query(query, &points, &indicies, height as usize - 10)?;
    };
    let items: Vec<Paper> = indicies
        .iter()
        .filter_map(|key| papers.get(key).cloned())
        .collect();
    let paper = match prompt_select(&items)? {
        Some(index) => Some(items[index].clone()),
        None => None,
    };
    Ok(paper)
}

pub fn open(query: String) -> Result<()> {
    let mut papers = load_papers()?;
    match select(query, &papers)? {
        Some(paper) => {
            paper.open_pdf()?;
            pull_up(&mut papers, &paper.id);
            save_papers(&papers)?;
        }
        None => (),
    };
    Ok(())
}

pub fn yank(query: String) -> Result<()> {
    let mut papers = load_papers()?;
    match select(query, &papers)? {
        Some(paper) => {
            let mut ctx = ClipboardContext::new()
                .map_err(|e| anyhow!("Failed to create clipboard context: {}", e))?;
            ctx.set_contents(paper.bibtex.clone())
                .map_err(|e| anyhow!("Failed to set clipboard contents: {}", e))?;
            pull_up(&mut papers, &paper.id);
            save_papers(&papers)?;
            blog!("Copied", "bibtex to clipboard")
        }
        None => (),
    };
    Ok(())
}

fn toggle_paper_stack(paper: &mut Paper, new_stack: &Stack) {
    let stack_index = paper.stack.iter().position(|s| s.name == new_stack.name);
    match stack_index {
        Some(index) => {
            // Stack exists, so remove it
            paper.stack.remove(index);
        }
        None => {
            // Stack doesn't exist, so add it
            paper.stack.push(new_stack.clone());
        }
    }
}

pub fn toggle(stack: String, query: String) -> Result<()> {
    let config = read_config_file()?;
    let stack = config
        .stacks
        .iter()
        .find(|&s| s.name == stack)
        .ok_or(anyhow!("Stack {} does not exist", stack))?;
    let mut papers = load_papers()?;
    let points = load_vectors()?;
    let (_width, height) = termion::terminal_size()?;
    let mut indicies = filter_by_stack(&papers)?;
    if query.len() > 0 {
        indicies = filter_by_query(query, &points, &indicies, height as usize - 10)?;
    };
    let items: Vec<Paper> = indicies
        .iter()
        .filter_map(|key| papers.get(key).cloned())
        .collect();
    match prompt_select(&items)? {
        Some(index) => {
            let key = items[index].id.clone();
            let paper = papers.get_mut(&key).unwrap(); //this is totally safe
            toggle_paper_stack(paper, stack);
            pull_up(&mut papers, &key);
            save_papers(&papers)?;
            Ok(())
        }
        None => Ok(()),
    }
}
