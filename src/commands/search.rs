use crate::base::Paper;
use crate::commands::cleanup::delete_paper;
use crate::settings;
use crate::utils::ui::Item;
use crate::utils::{bibfile, ui};
use anyhow::{bail, Result};

fn open_pdf(paper: Paper) -> Result<()> {
    //TODO BRING TO THE TOP OF BIBLIOGRAPHY
    paper.open_pdf()
}

fn search_stack(query: String) -> Result<()> {
    let bibliography = bibfile::read_bibliography()?;
    let mut items = bibfile::parse_bibliography(bibliography);
    items.reverse();
    let stack = settings::current_stack()?;
    let action = format!("[{}]", stack.to_uppercase());
    let color = String::from("Yellow");
    match ui::display_list(action, color, items, query, false, true) {
        Some(action) => match action {
            ui::Action::Open(paper) => open_pdf(paper),
            ui::Action::Remove(paper) => delete_paper(paper),
        },
        None => Ok(()),
    }
}

fn immutable_search() -> Result<()> {
    let bibliography = bibfile::read_bibliography()?;
    let mut items = bibfile::parse_bibliography(bibliography);
    items.reverse();
    // Sort papers by last_accessed (more recent first)
    // SORT THE PAPERS! (maybe from top to bottom)
    // Select the first 5 elements
    let first_ten: Vec<_> = items.iter().take(10).cloned().collect();
    let stack = settings::current_stack()?;
    let action = format!("[{}]", stack.to_uppercase());
    let color = String::from("Red");
    match ui::display_list(action, color, first_ten, String::new(), false, false) {
        Some(action) => match action {
            ui::Action::Open(paper) => open_pdf(paper),
            _ => bail!("Action not allowed in immutable search"),
        },
        None => Ok(()),
    }
}
pub fn peek() {
    let result = immutable_search();
    match result {
        Ok(_) => (),
        Err(err) => println!("{}", err),
    }
}

pub fn search(query: String) {
    //Loading bigliography
    let result = search_stack(query);
    match result {
        Ok(_) => (),
        Err(err) => println!("{}", err),
    }
}

pub fn list() -> Result<()> {
    //Loading bigliography
    let (width, _) = termion::terminal_size().unwrap();
    let bibliography = bibfile::read_bibliography()?;
    let mut items = bibfile::parse_bibliography(bibliography);
    items.reverse();
    let n_refs = items.len();
    // Iterate over the first 20 elements or all elements if less than 20
    let max_entries = 20.min(n_refs);
    for i in 0..max_entries {
        println!("{}", items[i].display(width));
    }
    // Print a message if there are more than 20 entries not being displayed
    if n_refs > 20 {
        println!("\t ----- hiding other {} references -----", n_refs - 20);
    }
    Ok(())
}
