use crate::base::Paper;
use crate::commands::cleanup::delete_paper;
use crate::settings;
use crate::utils::ui::Item;
use crate::utils::{bibfile, ui};
use anyhow::Result;
use std::cmp;

fn open_pdf(paper: Paper) -> Result<()> {
    //TODO BRING TO THE TOP OF BIBLIOGRAPHY
    paper.open_pdf()
}

pub fn search(query: String) -> Result<()> {
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

pub fn list(max:Option<usize>) -> Result<()> {
    //Loading bigliography
    let (width, height) = termion::terminal_size()?;
    let bibliography = bibfile::read_bibliography()?;
    let mut items = bibfile::parse_bibliography(bibliography);
    items.reverse();
    let n_refs = items.len();

    // Determine the maximum number of entries to display
    let max_entries = match max {
        Some(m) => cmp::min(m, n_refs),
        None => cmp::min(height as usize - 4, n_refs),
    };

    // Iterate over the specified number of elements
    for i in 0..max_entries {
        println!("{}", items[i].display(width));
    }

    // Print a message if there are more references not being displayed
    if n_refs > max_entries {
        println!("\t ----- hiding other {} references -----", n_refs - max_entries);
    }
    Ok(())
}
