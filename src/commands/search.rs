use crate::base::Paper;
use crate::semantic;
use crate::utils::bibfile;
use crate::utils::ui;
use anyhow::anyhow;
use anyhow::Result;
use biblatex::Bibliography;

fn search_online(query: String, here: bool) -> Result<()> {
    let bibliography = bibfile::read_bibliography().expect("Unable to read bibliography");
    let action = String::from("[ONLINE]");
    let color = String::from("blue");
    let mut items = semantic::query::query_papers(&query, 100)?;
    //filter the ones that are already in the bibliography
    semantic::query::remove_already_present(bibliography, &mut items);
    //run ui
    match ui::run_ui(action, color, items, query, true, false, false) {
        Some(action) => match action {
            ui::Action::Submit(paper) => add_paper_to_library(paper, here),
            _ => Err(anyhow!("Action not allowed in online search")),
        },
        None => Ok(()),
    }
}

fn add_paper_to_library(paper: Paper, local: bool) -> Result<()> {
    let mut bibliography: Bibliography;
    if local {
        println!("Adding papers to LOCAL library: {}", paper.title);
        bibliography = match bibfile::read_local_bibliography() {
            Ok(local_bib) => local_bib,
            Err(_) => Bibliography::new(),
        }
    } else {
        println!("Adding papers to MAIN library: {}", paper.title);
        bibliography = bibfile::read_bibliography().expect("Unable to read main library");
    }
    bibliography.insert(paper.entry);
    bibfile::save_bibliography(bibliography, local)
}

fn search_local(query: String) -> Result<()> {
    //read bibliography from current directory
    let bibliography = bibfile::read_local_bibliography()?;
    let items = bibfile::parse_bibliography(bibliography);
    let action = String::from("[LOCAL]");
    let color = String::from("green");
    match ui::run_ui(action, color, items, query, false, true, true) {
        Some(action) => match action {
            ui::Action::Submit(paper) => open_notes(paper),
            ui::Action::Open(paper) => open_pdf(paper),
            ui::Action::Remove(paper) => delete_paper_main(paper),
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
    match ui::run_ui(action, color, items, query, false, true, true) {
        Some(action) => match action {
            ui::Action::Submit(paper) => open_notes(paper),
            ui::Action::Open(paper) => open_pdf(paper),
            ui::Action::Remove(paper) => delete_paper_main(paper),
        },
        None => (),
    }
    Ok(())
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
