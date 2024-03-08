use crate::settings;
use crate::utils::bibfile;
use anyhow::Result;
use biblatex::Person;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use termion::color;

struct Author(Person);

impl Hash for Author {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.name.hash(state);
        self.0.given_name.hash(state);
        self.0.prefix.hash(state);
        self.0.suffix.hash(state);
    }
}

impl PartialEq for Author {
    fn eq(&self, other: &Self) -> bool {
        self.0.name == other.0.name
            && self.0.given_name == other.0.given_name
            && self.0.prefix == other.0.prefix
            && self.0.suffix == other.0.suffix
    }
}

impl Eq for Author {}

pub fn status() -> Result<()> {
    // Calculate the total number of references
    let bibliography = bibfile::read_bibliography()?;
    let total_references = bibliography.len();

    // Count the occurrences of each author
    let mut author_counts: HashMap<Author, usize> = HashMap::new();
    for entry in bibliography.iter() {
        let authors = match entry.author() {
            Ok(authors) => authors,
            Err(_) => continue,
        };
        for author in authors {
            *author_counts.entry(Author(author)).or_insert(0) += 1;
        }
    }
    // Sort authors by the number of references
    let mut sorted_authors: Vec<_> = author_counts.into_iter().collect();
    sorted_authors.sort_by(|&(_, count1), &(_, count2)| count2.cmp(&count1));
    // Print total number of references
    let stack = settings::current_stack()?;
    println!(
        "Stack: {}{}{}",
        color::Fg(color::Yellow),
        stack,
        color::Fg(color::Reset),
    );
    println!("References: {}",total_references);
    // Print top authors
    println!("Top Authors:");
    for (i, (author, count)) in sorted_authors.iter().take(10).enumerate() {
        println!(
            "  {}{}.{} {} {} - {}",
            color::Fg(color::Yellow),
            i + 1,
            color::Fg(color::Reset),
            author.0.given_name,
            author.0.name,
            count,
        );
    }
    Ok(())
}
