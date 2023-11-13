use crate::base::Paper;
use crate::commands::add::add_online_paper;
use crate::semantic::query;
use crate::settings::{self, PDF_VIEWER, QUERY_LIMIT};
use crate::utils::{bibfile, ui};
use anyhow::anyhow;
use anyhow::Result;
use biblatex::Bibliography;
use std::path::Path;
use std::process::{exit, Command};

fn open_pdf_in_subprocess(pdf_path: &str) {
    // Use the "zathura" command to open the PDF file
    println!("{}", pdf_path);
    let result = Command::new(PDF_VIEWER).arg(pdf_path).spawn();

    match result {
        Ok(_) => {
            println!("Opened PDF with zathura");
        }
        Err(err) => {
            eprintln!("Error opening PDF with zathura: {}", err);
            exit(1);
        }
    }
}
fn remove_already_present(bibfile: Bibliography, papers: &mut Vec<Paper>) {
    papers.retain(|paper| bibfile.get(&paper.entry.key).is_none());
}
fn show_notes(paper: Paper) {
    println!("Opening notes on paper: {}", paper.title);
}
fn open_pdf(paper: Paper) -> Result<()> {
    let directory = settings::pdf_dir()?;
    let filename = format!("{}.pdf", paper.id); // Use format! to create the filename
    let file_path = Path::new(&directory).join(&filename);
    // Check if the file already exists
    if !file_path.exists() {
        return Err(anyhow!("No PDF {} found on current stack\nAdd it manually with `bib add`", &filename));
    };
    let pdf_path = directory.to_string() + &filename;
    open_pdf_in_subprocess(&pdf_path);
    Ok(())
}

fn delete_paper(paper: Paper) {
    println!("Deleting paper from MAIN: {}", paper.title);
    //search for pdf
}

fn search_online(query: String) -> Result<()> {
    let bibliography = bibfile::read_bibliography().expect("Unable to read bibliography");
    let action = String::from("[ONLINE]");
    let color = String::from("blue");
    let mut items = query::query_batch_papers(&query, QUERY_LIMIT)?;
    //filter the ones that are already in the bibliography
    remove_already_present(bibliography, &mut items);
    match ui::display_list(action, color, items, query, true, false, false) {
        Some(action) => match action {
            ui::Action::Submit(paper) => add_online_paper(paper),
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
    match ui::display_list(action, color, items, query, false, true, true) {
        Some(action) => match action {
            ui::Action::Open(paper) => Ok(show_notes(paper)),
            ui::Action::Submit(paper) => open_pdf(paper),
            ui::Action::Remove(paper) => Ok(delete_paper(paper)),
        },
        None => Ok(()),
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
