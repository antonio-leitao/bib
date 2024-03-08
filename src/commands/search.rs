use crate::base::Paper;
use crate::commands::cleanup::delete_paper;
use crate::settings;
// use crate::utils::ui::Item;
use crate::utils::{bibfile, ui};
use anyhow::Result;

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

// pub fn list() -> Result<()> {
//     //Loading bigliography
//     let (width, _) = termion::terminal_size().unwrap();
//     let bibliography = bibfile::read_bibliography()?;
//     let mut items = bibfile::parse_bibliography(bibliography);
//     items.reverse();
//     let n_refs = items.len();
//     // Iterate over the first 20 elements or all elements if less than 20
//     let max_entries = 20.min(n_refs);
//     for i in 0..max_entries {
//         println!("{}", items[i].display(width));
//     }
//     // Print a message if there are more than 20 entries not being displayed
//     if n_refs > 20 {
//         println!("\t ----- hiding other {} references -----", n_refs - 20);
//     }
//     Ok(())
// }
