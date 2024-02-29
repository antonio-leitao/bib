use crate::base::Paper;
use crate::utils::bibfile;
use anyhow::Result;
use biblatex::Bibliography;

fn remove_from_bibliography(key: &str) -> Result<()> {
    //add to main bibliography
    let mut bibliography: Bibliography;
    bibliography = bibfile::read_bibliography()?;
    bibliography.remove(&key);
    bibfile::save_bibliography(bibliography)
}

pub fn delete_paper(paper: Paper) -> Result<()> {
    println!("Deleting paper and notes from: {}", paper.title);
    remove_from_bibliography(&paper.id)
}
