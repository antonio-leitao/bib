use crate::semantic;
use crate::utils::bibfile::{parse_bibliography, read_bibliography};
use crate::utils::ui;
use anyhow::Result;
use biblatex::Bibliography;

fn search_online(bibliography: Bibliography, query: String, here: bool) -> Result<()> {
    let action = String::from("[ONLINE]");
    let color = String::from("blue");
    let mut items = semantic::query::query_papers(&query, 100)?;
    //filter the ones that are already in the bibliography
    semantic::query::remove_already_present(bibliography, &mut items);
    match ui::run_ui(action, color, items, query, true) {
        Some(paper) => println!("{}", paper.title),
        None => (),
    }
    Ok(())
}

fn search_here(query: String) -> Result<()> {
    //read bibliography from current directory
    //parse into items
    //run ui
    println!("To implement");
    Ok(())
}

fn search_local(bibliography: Bibliography, query: String) -> Result<()> {
    let action = String::from("[LIBRARY]");
    let color = String::from("red");
    let items = parse_bibliography(bibliography);
    match ui::run_ui(action, color, items, query, false) {
        Some(paper) => println!("{}", paper.title),
        None => (),
    }
    Ok(())
}

pub fn search(query: String, online: bool, here: bool) {
    //Loading bigliography
    let bibliography = read_bibliography().expect("Unable to read bibliography");
    let result = match (online, here) {
        (true, _) => search_online(bibliography, query, here),
        (false, true) => search_here(query),
        (false, false) => search_local(bibliography, query),
    };
}
