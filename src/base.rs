use crate::utils::ui;
use anyhow::{bail, Result};
use biblatex::Entry;
use open;
use termion::color;

#[derive(Clone)]
pub struct Paper {
    pub id: String,
    pub author: String,
    pub year: i64,
    pub title: String,
    pub slug: String,
    pub url: Option<String>,
    pub entry: Entry,
}

impl Paper {
    pub fn open_pdf(&self) -> Result<()> {
        match &self.url {
            Some(url) => match open::that(url) {
                Ok(()) => Ok(()),
                Err(err) => bail!("Could not open pdf {}", err),
            },
            None => bail!("Selected paper has no available url"),
        }
    }
}

impl Paper {
    fn get_slack(&self) -> usize {
        if self.url.is_some() {
            return 6;
        }
        return 0;
    }
    fn trim_title(&self, max_length: u16) -> String {
        let mut length = max_length as usize;
        length -= 4 + 2;
        length -= self.author.len() + 4;
        length -= self.get_slack();
        fit_string_to_length(&self.title, length)
    }
}

impl ui::Item for Paper {
    fn display(&self, max_width: u16) -> String {
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
        if self.url.is_some() {
            display_string.push_str(&format!(
                " {}[PDF]{}",
                color::Fg(color::Red),
                color::Fg(color::Reset)
            ));
        }
        display_string
    }

    fn disabled(&self, max_width: u16) -> String {
        // let slack = self.get_slack();
        let mut disabled_string = format!(
            "{}  {} | {} | {}",
            color::Fg(color::Rgb(83, 110, 122)),
            self.year,
            self.author,
            self.trim_title(max_width),
        );

        if self.url.is_some() {
            disabled_string.push_str(&format!(
                " {}[PDF]{}",
                color::Fg(color::Red),
                color::Fg(color::Reset)
            ));
        }
        disabled_string
    }
    fn slug(&self) -> String {
        self.slug.clone()
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

