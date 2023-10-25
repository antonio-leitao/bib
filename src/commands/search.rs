use crate::base::Paper;
use crate::commands::add::add_paper_to_library;
use crate::semantic::query;
use crate::settings::QUERY_LIMIT;
use crate::utils::{bibfile, ui};
use anyhow::anyhow;
use anyhow::Result;
use biblatex::Bibliography;
fn remove_already_present(bibfile: Bibliography, papers: &mut Vec<Paper>) {
    papers.retain(|paper| bibfile.get(&paper.entry.key).is_none());
}
fn open_notes(paper: Paper) {
    println!("Opening notes on paper: {}", paper.title);
}
fn open_pdf(paper: Paper) {
    println!("Opening pdf on paper: {}", paper.title);
}
fn delete_paper_main(paper: Paper) {
    println!("Deleting paper from MAIN: {}", paper.title);
}
fn delete_paper_local(paper: Paper) {
    println!("Deleting paper from MAIN: {}", paper.title);
}
fn search_online(query: String, here: bool) -> Result<()> {
    let bibliography = bibfile::read_bibliography().expect("Unable to read bibliography");
    let action = String::from("[ONLINE]");
    let color = String::from("blue");
    let mut items = query::query_batch_papers(&query, QUERY_LIMIT)?;
    //filter the ones that are already in the bibliography
    remove_already_present(bibliography, &mut items);
    //run ui
    match ui::display_list(action, color, items, query, true, false, false) {
        Some(action) => match action {
            ui::Action::Submit(paper) => {
                ui::display_spinner(move || add_paper_to_library(paper, here), "Adding papers")
            }
            _ => Err(anyhow!("Action not allowed in online search")),
        },
        None => Ok(()),
    }
}

fn search_local(query: String) -> Result<()> {
    //read bibliography from current directory
    let bibliography = bibfile::read_local_bibliography()?;
    let items = bibfile::parse_bibliography(bibliography);
    let action = String::from("[LOCAL]");
    let color = String::from("green");
    match ui::display_list(action, color, items, query, false, true, true) {
        Some(action) => match action {
            ui::Action::Submit(paper) => open_notes(paper),
            ui::Action::Open(paper) => open_pdf(paper),
            ui::Action::Remove(paper) => delete_paper_local(paper),
        },
        None => (),
    }
    Ok(())
}

fn search_main(query: String) -> Result<()> {
    let bibliography = bibfile::read_bibliography()?;
    let items = bibfile::parse_bibliography(bibliography);
    let action = String::from("[MAIN]");
    let color = String::from("red");
    match ui::display_list(action, color, items, query, false, true, true) {
        Some(action) => match action {
            ui::Action::Submit(paper) => open_notes(paper),
            ui::Action::Open(paper) => open_pdf(paper),
            ui::Action::Remove(paper) => delete_paper_main(paper),
        },
        None => (),
    }
    Ok(())
}

pub fn search(query: String, online: bool, here: bool) {
    //Loading bigliography
    let result = match (online, here) {
        (true, _) => search_online(query, here),
        (false, true) => search_local(query),
        (false, false) => search_main(query),
    };
    match result {
        Ok(_) => (),
        Err(err) => println!("{}", err),
    }
}
