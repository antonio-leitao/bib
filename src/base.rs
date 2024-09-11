use crate::stacks::Stack;
use crate::utils;
use anyhow::{anyhow, Result};
use bincode;
use indexmap::IndexMap;
use open;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use termion::color;

#[derive(Clone, Debug, Serialize, Deserialize)] // TODO: Why do we need this clone?
pub struct Paper {
    pub id: String,
    pub author: String,
    pub year: i64,
    pub title: String,
    pub stack: Vec<Stack>,
    pub bibtex: String,
}

impl Paper {
    pub fn open_pdf(&self) -> Result<()> {
        let pdf_path = utils::io::pdf_path(&self.id)?;
        open::that(pdf_path).map_err(|err| anyhow!("Could not open pdf: {}", err))
    }
    fn get_slack(&self) -> usize {
        self.stack
            .iter()
            .fold(0, |acc, stack| acc + stack.name.len() + 3)
    }
    fn trim_title(&self, max_length: u16) -> String {
        let mut length = max_length as usize;
        length -= 4 + 2;
        length -= self.author.len() + 4;
        length -= self.get_slack();
        fit_string_to_length(&self.title, length)
    }
    pub fn display(&self, max_width: u16) -> String {
        // let slack = self.get_slack();
        let mut display_string = format!(
            "{} {}|{} {} {}|{} {}",
            self.year,
            color::Fg(color::Rgb(83, 110, 122)),
            color::Fg(color::Reset),
            self.author,
            color::Fg(color::Rgb(83, 110, 122)),
            color::Fg(color::Reset),
            self.trim_title(max_width),
        );
        // display_string = fit_string_to_length(display_string, max_width - slack);
        for stack in self.stack.iter() {
            display_string.push_str(&format!(" {}", stack));
        }
        display_string
    }
}

fn fit_string_to_length(input: &str, max_length: usize) -> String {
    if input.len() <= max_length {
        return String::from(input);
    }

    let mut result = String::with_capacity(max_length);
    result.push_str(&input[..max_length - 3]);
    result.push_str("...");
    result
}

pub fn save_papers(papers: &IndexMap<String, Paper>) -> Result<()> {
    let encoded: Vec<u8> = bincode::serialize(papers)?;
    let filename = utils::io::papers_path()?;
    let mut file = File::create(filename)?;
    file.write_all(&encoded)?;
    Ok(())
}

pub fn load_papers() -> Result<IndexMap<String, Paper>> {
    let filename = utils::io::papers_path()?;
    if !filename.exists() {
        return Ok(IndexMap::new());
    }
    let mut file = File::open(filename)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let decoded: IndexMap<String, Paper> = bincode::deserialize(&buffer)?;
    Ok(decoded)
}
