use crate::base::Paper;
use crate::store::{PaperStore, StoreError};
use std::io::{self, Stdout, Write};
use sublime_fuzzy::best_match;
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Store error: {0}")]
    Store(#[from] StoreError),
    #[error("UI error: {0}")]
    IOError(#[from] io::Error),
}

// Mode enum
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Search,
    Browse,
}

impl Mode {
    fn toggle(&self) -> Self {
        match self {
            Mode::Search => Mode::Browse,
            Mode::Browse => Mode::Search,
        }
    }
}

// Message enum for updates (Elm-like architecture)
enum Message {
    ModeToggle,
    SearchInput(char),
    SearchBackspace,
    BrowseUp,
    BrowseDown,
    Quit,
    // Future messages can be added here:
    // SelectPaper,
    // OpenPaper,
    // DeletePaper,
    // etc.
}

// Main UI struct that holds all state
struct SearchUI<'a> {
    papers: Vec<Paper>,
    query: String,
    mode: Mode,
    cursor_pos: usize,
    limit: usize,
    stdout: RawTerminal<Stdout>,
    store: &'a PaperStore,
}

impl<'a> SearchUI<'a> {
    // Initialize the UI
    fn init(store: &'a PaperStore, limit: usize) -> Result<Self, SearchError> {
        let papers = store.list_all(None)?;
        let mut stdout = io::stdout().into_raw_mode()?;

        // Hide cursor
        write!(stdout, "{}", termion::cursor::Hide)?;

        let mut ui = SearchUI {
            papers,
            query: String::new(),
            mode: Mode::Search,
            cursor_pos: 0,
            limit,
            stdout,
            store,
        };

        // Initial render
        ui.view()?;

        Ok(ui)
    }

    // Main event loop
    fn run(mut self) -> Result<(), SearchError> {
        let stdin = io::stdin();

        for key in stdin.keys() {
            match self.handle_key(key.unwrap()) {
                Some(Message::Quit) => break,
                Some(msg) => {
                    self.update(msg)?;
                    self.view()?;
                }
                None => {} // No action needed
            }
        }
        self.cleanup()?;
        Ok(())
    }

    // Convert key press to message
    fn handle_key(&self, key: Key) -> Option<Message> {
        // First check for universal commands (work in any mode)
        match key {
            Key::Char('\n') | Key::Esc | Key::Ctrl('c') => return Some(Message::Quit),
            Key::Char('\t') | Key::Char('\\') => return Some(Message::ModeToggle),
            _ => {}
        }

        // Then delegate to mode-specific handlers
        match self.mode {
            Mode::Search => self.handle_key_search(key),
            Mode::Browse => self.handle_key_browse(key),
        }
    }

    // Handle keys specific to search mode
    fn handle_key_search(&self, key: Key) -> Option<Message> {
        match key {
            Key::Char(ch) => Some(Message::SearchInput(ch)),
            Key::Backspace => Some(Message::SearchBackspace),
            _ => None,
        }
    }

    // Handle keys specific to browse mode
    fn handle_key_browse(&self, key: Key) -> Option<Message> {
        match key {
            Key::Char('j') | Key::Down => Some(Message::BrowseDown),
            Key::Char('k') | Key::Up => Some(Message::BrowseUp),
            // Future browse commands can be added here:
            // Key::Char(' ') => Some(Message::ToggleSelection),
            // Key::Char('o') => Some(Message::OpenPaper),
            // Key::Char('d') => Some(Message::DeletePaper),
            // Key::Char('y') => Some(Message::CopyPaper),
            _ => None,
        }
    }

    // Update state based on message
    fn update(&mut self, msg: Message) -> Result<(), SearchError> {
        match msg {
            Message::ModeToggle => {
                self.mode = self.mode.toggle();
                if self.mode == Mode::Browse {
                    self.cursor_pos = 0;
                }
            }
            Message::SearchInput(ch) => {
                self.query.push(ch);
            }
            Message::SearchBackspace => {
                self.query.pop();
            }
            Message::BrowseUp => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            Message::BrowseDown => {
                let results = self.get_filtered_papers();
                if self.cursor_pos < results.len().saturating_sub(1) {
                    self.cursor_pos += 1;
                }
            }
            Message::Quit => {} // Handled in run()
                                // Future message handlers can be added here
        }
        Ok(())
    }

    // Render the UI
    fn view(&mut self) -> Result<(), SearchError> {
        // Get filtered papers and extract the data we need before borrowing stdout
        let results = self.get_filtered_papers();
        let total_matches = self.count_all_matches();
        let (width, _) = termion::terminal_size()?;
        let result_displays: Vec<String> = results
            .iter()
            .map(|paper| paper.display(width.saturating_sub(4)))
            .collect();
        let num_results = result_displays.len();
        let hidden_count = total_matches.saturating_sub(self.limit);

        // Show current mode and query
        let mode_indicator = match self.mode {
            Mode::Search => "[SEARCH]",
            Mode::Browse => "[BROWSE]",
        };

        writeln!(
            self.stdout,
            "{}{} > {}\r",
            termion::clear::CurrentLine,
            mode_indicator,
            self.query
        )?;

        // Print results with cursor in browse mode
        for (i, display) in result_displays.iter().enumerate() {
            write!(self.stdout, "{}", termion::clear::CurrentLine)?;

            // Show cursor (*) in browse mode
            if self.mode == Mode::Browse && i == self.cursor_pos {
                write!(self.stdout, "* ")?;
            } else {
                write!(self.stdout, "  ")?;
            }

            write!(self.stdout, "{}", display)?;
            writeln!(self.stdout, "\r")?;
        }

        // Print blank lines for remaining slots
        for i in num_results..self.limit {
            write!(self.stdout, "{}", termion::clear::CurrentLine)?;

            // Show cursor even on empty lines if in browse mode
            if self.mode == Mode::Browse && i == self.cursor_pos {
                write!(self.stdout, "* ")?;
            } else {
                write!(self.stdout, "  ")?;
            }

            writeln!(self.stdout, "\r")?;
        }

        // Show hidden results count if applicable
        write!(self.stdout, "{}", termion::clear::CurrentLine)?;
        if hidden_count > 0 {
            writeln!(
                self.stdout,
                "{}  ... {} more results hidden{}\r",
                color::Fg(color::Rgb(83, 110, 122)),
                hidden_count,
                color::Fg(color::Reset)
            )?;
        } else {
            writeln!(self.stdout, "\r")?; // Empty line
        }

        // Show commands based on mode
        write!(self.stdout, "{}", termion::clear::CurrentLine)?;
        let commands = match self.mode {
            Mode::Search => "Tab: Browse | Esc: Quit | Type to search",
            Mode::Browse => "Tab: Search | j/↓: Down | k/↑: Up | Esc: Quit",
        };
        writeln!(self.stdout, "  {}\r", commands)?;

        // Move cursor back up (now accounting for 2 extra lines)
        write!(
            self.stdout,
            "{}",
            termion::cursor::Up(self.limit as u16 + 3)
        )?;
        self.stdout.flush()?;
        Ok(())
    }

    // Get filtered papers based on current query (with limit)
    fn get_filtered_papers(&self) -> Vec<&Paper> {
        fuzzy_search_papers(&self.papers, &self.query, self.limit)
    }

    // Count total matches (more efficient than getting all papers)
    fn count_all_matches(&self) -> usize {
        if self.query.trim().is_empty() {
            self.papers.len()
        } else {
            self.papers
                .iter()
                .filter(|paper| best_match(&self.query, &paper.content).is_some())
                .count()
        }
    }

    // Get currently selected paper (for future use)
    #[allow(dead_code)]
    fn get_selected_paper(&self) -> Option<&Paper> {
        if self.mode == Mode::Browse {
            self.get_filtered_papers().get(self.cursor_pos).copied()
        } else {
            None
        }
    }

    // Cleanup when exiting
    fn cleanup(&mut self) -> Result<(), SearchError> {
        write!(
            self.stdout,
            "{}{}",
            termion::clear::AfterCursor,
            termion::cursor::Show
        )?;
        Ok(())
    }
}

// Main fuzzy search function
fn fuzzy_search_papers<'a>(papers: &'a [Paper], query: &str, limit: usize) -> Vec<&'a Paper> {
    // Handle empty query - return first `limit` papers
    if query.trim().is_empty() {
        return papers.iter().take(limit).collect();
    }

    // Compute scores once and collect matches
    let mut scored_papers: Vec<(isize, &Paper)> = papers
        .iter()
        .filter_map(|paper| {
            best_match(query, &paper.content).map(|fuzzy_match| (fuzzy_match.score(), paper))
        })
        .collect();

    // Sort by score descending
    scored_papers.sort_unstable_by(|a, b| b.0.cmp(&a.0));

    // Take top results
    scored_papers
        .into_iter()
        .take(limit)
        .map(|(_, paper)| paper)
        .collect()
}

// Public entry point
pub fn interactive_search(store: &PaperStore, limit: usize) -> Result<(), SearchError> {
    let ui = SearchUI::init(store, limit)?;
    ui.run()
}
// pub fn interactive_search(store: &PaperStore, limit: usize) -> Result<(), SearchError> {
//     let papers = store.list_all(None)?;
//     let stdin = io::stdin();
//     let mut stdout = io::stdout().into_raw_mode().unwrap();
//     // let (width, _) = termion::terminal_size().unwrap();
//     let mut query = String::new();
//     // Move the cursor to the bottom of the previous output before starting
//     //hide cursor
//     write!(stdout, "{}", termion::cursor::Hide)?;
//     draw_search_ui(&mut stdout, &query, &papers, limit)?;
//
//     for c in stdin.keys() {
//         match c.unwrap() {
//             Key::Char('\n') => {
//                 break;
//             }
//             Key::Esc | Key::Ctrl('c') => {
//                 break;
//             }
//             Key::Char(ch) => {
//                 query.push(ch); // Add character to query
//                 draw_search_ui(&mut stdout, &query, &papers, limit)?; // Redraw UI
//             }
//             Key::Backspace => {
//                 query.pop(); // Remove last character
//                 draw_search_ui(&mut stdout, &query, &papers, limit)?; // Redraw UI
//             }
//             _ => {}
//         }
//     }
//     // Clean screen
//     write!(
//         stdout,
//         "{}{}",
//         termion::clear::AfterCursor,
//         termion::cursor::Show
//     )?;
//     Ok(())
// }
//
// fn draw_search_ui(
//     stdout: &mut RawTerminal<Stdout>,
//     query: &str,
//     papers: &[Paper],
//     limit: usize,
// ) -> Result<(), SearchError> {
//     let results = fuzzy_search_papers(papers, query, limit);
//     let (width, _) = termion::terminal_size().unwrap();
//
//     // Print the actual results
//     writeln!(stdout, "{}> {}\r", termion::clear::CurrentLine, query)?;
//     for (_i, word) in results.iter().enumerate() {
//         write!(
//             stdout,
//             "{}{}",
//             termion::clear::CurrentLine,
//             word.display(width - 2)
//         )?;
//         writeln!(stdout, "\r")?;
//     }
//
//     // Print blank lines for remaining slots
//     for _ in results.len()..limit {
//         write!(stdout, "{}", termion::clear::CurrentLine)?;
//         writeln!(stdout, "\r")?;
//     }
//
//     // Move cursor back up by the full limit, not just results.len()
//     write!(stdout, "{}", termion::cursor::Up(limit as u16 + 1))?;
//     stdout.flush()?;
//     Ok(())
// }
//
// pub fn open(query: String) -> Result<()> {
//     let mut papers = load_papers()?;
//     match select(query, &papers)? {
//         Some(paper) => {
//             paper.open_pdf()?;
//             pull_up(&mut papers, &paper.id);
//             save_papers(&papers)?;
//         }
//         None => (),
//     };
//     Ok(())
// }
//
// pub fn yank(query: String) -> Result<()> {
//     let mut papers = load_papers()?;
//     match select(query, &papers)? {
//         Some(paper) => {
//             let mut ctx = ClipboardContext::new()
//                 .map_err(|e| anyhow!("Failed to create clipboard context: {}", e))?;
//             ctx.set_contents(paper.bibtex.clone())
//                 .map_err(|e| anyhow!("Failed to set clipboard contents: {}", e))?;
//             pull_up(&mut papers, &paper.id);
//             save_papers(&papers)?;
//             blog!("Copied", "bibtex to clipboard")
//         }
//         None => (),
//     };
//     Ok(())
// }
