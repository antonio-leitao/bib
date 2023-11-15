use crate::base::Paper;
use crate::commands::add::{add_online_paper, insert_metadata};
use crate::commands::cleanup::delete_paper;
use crate::semantic::query;
use crate::settings::{self, EDITOR, QUERY_LIMIT};
use crate::utils::{bibfile, ui};
use anyhow::anyhow;
use anyhow::Result;
use biblatex::Bibliography;
use std::fs;
use std::path::Path;
use std::process::Command;

fn remove_already_present(bibfile: Bibliography, papers: &mut Vec<Paper>) {
    papers.retain(|paper| bibfile.get(&paper.entry.key).is_none());
}

fn open_notes(mut paper: Paper) -> Result<()> {
    paper.update_last_accessed();
    if let Some(mut meta) = paper.meta {
        let directory = settings::notes_dir()?;
        let filename = format!("{}.txt", paper.id);
        let file_path = Path::new(&directory).join(&filename);
        match meta.notes {
            None => {
                //create and update metadata
                fs::create_dir_all(&directory)?;
                Command::new(EDITOR).arg(file_path).status()?;
                meta.notes = Some(filename);
                insert_metadata(paper.id, meta)?;
            }
            Some(_) => {
                Command::new(EDITOR).arg(file_path).status()?;
                insert_metadata(paper.id, meta)?;
            }
        }
    };
    Ok(())
}

fn open_local_pdf(mut paper: Paper) -> Result<()> {
    paper.update_last_accessed();
    if let Some(meta) = paper.meta {
        let pdf = meta.pdf.clone();
        insert_metadata(paper.id, meta)?;
        if let Some(url) = pdf {
            url.open()?
        }
    };
    Ok(())
}
fn open_online_pdf(paper: Paper) -> Result<()> {
    if let Some(meta) = paper.meta {
        if let Some(pdf) = meta.pdf {
            pdf.open()?
        }
    };
    Ok(())
}

fn search_online(query: String) -> Result<()> {
    let bibliography = bibfile::read_bibliography().expect("Unable to read bibliography");
    let action = String::from("[ONLINE]");
    let color = String::from("blue");
    let mut items = query::query_batch_papers(&query, QUERY_LIMIT)?;
    //filter the ones that are already in the bibliography
    remove_already_present(bibliography, &mut items);
    match ui::display_list(action, color, items, query, true, true, false, false) {
        Some(action) => match action {
            ui::Action::Add(paper) => add_online_paper(paper),
            ui::Action::Open(paper) => open_online_pdf(paper),
            _ => Err(anyhow!("Action not allowed in online search")),
        },
        None => Ok(()),
    }
}

fn search_stack(query: String) -> Result<()> {
    let bibliography = bibfile::read_bibliography()?;
    let items = bibfile::parse_bibliography(bibliography);
    let stack = settings::current_stack()?;
    let action = format!("[{}]", stack.to_uppercase());
    let color = String::from("Yellow");
    match ui::display_list(action, color, items, query, false, false, true, true) {
        Some(action) => match action {
            ui::Action::Open(paper) => open_local_pdf(paper),
            ui::Action::Notes(paper) => open_notes(paper),
            ui::Action::Remove(paper) => delete_paper(paper),
            _ => Err(anyhow!("Action not allowed in local search")),
        },
        None => Ok(()),
    }
}

fn immutable_search() -> Result<()> {
    let bibliography = bibfile::read_bibliography()?;
    let mut items = bibfile::parse_bibliography(bibliography);
    // Sort papers by last_accessed (more recent first)
    items.sort_by(|a, b| {
        let a_timestamp = a
            .meta
            .as_ref()
            .and_then(|m| m.last_accessed)
            .unwrap_or(u64::MAX);
        let b_timestamp = b
            .meta
            .as_ref()
            .and_then(|m| m.last_accessed)
            .unwrap_or(u64::MAX);
        // Sort in descending order (more recent first)
        b_timestamp.cmp(&a_timestamp)
    });
    // Select the first 5 elements
    let first_ten: Vec<_> = items.iter().take(10).cloned().collect();
    let stack = settings::current_stack()?;
    let action = format!("[{}]", stack.to_uppercase());
    let color = String::from("Red");
    match ui::display_list(
        action,
        color,
        first_ten,
        String::new(),
        false,
        false,
        true,
        false,
    ) {
        Some(action) => match action {
            ui::Action::Notes(paper) => open_notes(paper),
            ui::Action::Open(paper) => open_local_pdf(paper),
            _ => Err(anyhow!("Action not allowed in online search")),
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

pub fn search(query: String, online: bool) {
    //Loading bigliography
    let result = match online {
        true => search_online(query),
        false => search_stack(query),
    };
    match result {
        Ok(_) => (),
        Err(err) => println!("{}", err),
    }
}
