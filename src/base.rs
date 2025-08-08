// use crate::stacks::Stack;
// use crate::utils;
use crate::bibtex::{self, BibtexError};
use serde::{Deserialize, Serialize};
use termion::color;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Paper {
    pub id: u128,
    pub key: String,
    pub author: String,
    pub year: i64,
    pub title: String,
    pub notes: Option<String>,
    pub bibtex: String,
}
impl Paper {
    pub fn from_bibtex(bibtex_str: String, notes: Option<String>) -> Result<Self, BibtexError> {
        // Parse the BibTeX string to get BibtexData
        let bibtex_data = bibtex::process_bibtex_entry(&bibtex_str)?;

        // Convert BibtexData to Paper
        Ok(Paper {
            id: bibtex_data.content_id, // Convert u128 to u64 by truncating
            key: bibtex_data.key,
            author: bibtex_data.author,
            year: bibtex_data.year,
            title: bibtex_data.title,
            notes,              // Start with no notes
            bibtex: bibtex_str, // Store the original BibTeX string
        })
    }
    pub fn display(&self) -> String {
        let mut display_string = format!(
            "{} {}|{} {} {}| ",
            self.year,
            color::Fg(color::Rgb(83, 110, 122)),
            color::Fg(color::Reset),
            self.author,
            color::Fg(color::Rgb(83, 110, 122)),
        );
        display_string.push_str(&format!(
            "{}{}",
            color::Fg(color::Reset),
            self.trim_details(&self.title, 80),
        ));
        display_string
    }
    fn trim_details(&self, details: &str, max_length: u16) -> String {
        let mut length = max_length as usize;
        length -= 4 + 2;
        length -= self.author.len() + 4;
        fit_string_to_length(details, length)
    }
}

// impl Paper {
//     pub fn open_pdf(&self) -> Result<()> {
//         let pdf_path = utils::io::pdf_path(&self.id)?;
//         open::that(pdf_path).map_err(|err| anyhow!("Could not open pdf: {}", err))
//     }
//     fn get_slack(&self) -> usize {
//         self.stack
//             .iter()
//             .fold(0, |acc, stack| acc + stack.name.len() + 3)
//     }
//     fn trim_details(&self, details: &str, max_length: u16) -> String {
//         let mut length = max_length as usize;
//         length -= 4 + 2;
//         length -= self.author.len() + 4;
//         length -= self.get_slack();
//         fit_string_to_length(details, length)
//     }
//     pub fn display(&self, max_width: u16, display_notes: bool) -> String {
//         let mut display_string = format!(
//             "{} {}|{} {} {}| ",
//             self.year,
//             color::Fg(color::Rgb(83, 110, 122)),
//             color::Fg(color::Reset),
//             self.author,
//             color::Fg(color::Rgb(83, 110, 122)),
//         );
//
//         // Only display notes if display_notes is true AND notes field contains a value
//         if display_notes && self.notes.is_some() {
//             display_string.push_str(&format!(
//                 "{}{}",
//                 self.trim_details(self.notes.as_ref().unwrap(), max_width),
//                 color::Fg(color::Reset)
//             ))
//         } else {
//             // Default to showing title
//             display_string.push_str(&format!(
//                 "{}{}",
//                 color::Fg(color::Reset),
//                 self.trim_details(&self.title, max_width),
//             ))
//         }
//
//         for stack in self.stack.iter() {
//             display_string.push_str(&format!(" {}", stack));
//         }
//         display_string
//     }
// }
//
fn fit_string_to_length(input: &str, max_length: usize) -> String {
    if input.len() <= max_length {
        return String::from(input);
    }

    let mut result = String::with_capacity(max_length);
    result.push_str(&input[..max_length - 3]);
    result.push_str("...");
    result
}
