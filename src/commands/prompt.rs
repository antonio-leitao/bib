use crate::base::Paper;
use crate::store::{PaperStore, StoreError};
use std::io::{self, Stdout, Write};
use std::time::{Duration, Instant};
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

// Message state for displaying feedback and prompts
enum MessageState {
    Flash {
        text: String,
        line_index: usize,
        expires_at: Instant,
    },
    Prompt {
        text: String,
        line_index: usize,
        action: PendingAction,
    },
}

// Actions that require confirmation
enum PendingAction {
    Delete(usize), // paper index in filtered results
}

// Message enum for updates (Elm-like architecture)
enum Message {
    ModeToggle,
    SearchInput(char),
    SearchBackspace,
    BrowseUp,
    BrowseDown,
    YankBibtex,
    DeletePaper,
    OpenPaper(bool), // bool flag for alt mode
    ConfirmPrompt,
    CancelPrompt,
    ClearMessage,
    Quit,
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
    message: Option<MessageState>,
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
            message: None,
        };

        // Initial render
        ui.view()?;

        Ok(ui)
    }

    // Main event loop
    fn run(mut self) -> Result<(), SearchError> {
        let stdin = io::stdin();

        for key in stdin.keys() {
            // Check for expired flash messages
            if let Some(MessageState::Flash { expires_at, .. }) = &self.message {
                if Instant::now() >= *expires_at {
                    self.message = None;
                    self.view()?;
                }
            }

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
        // First check if we're in a prompt
        if let Some(MessageState::Prompt { .. }) = &self.message {
            return match key {
                Key::Char('y') | Key::Char('Y') => Some(Message::ConfirmPrompt),
                Key::Char('n') | Key::Char('N') | Key::Esc => Some(Message::CancelPrompt),
                _ => None, // Ignore other keys during prompt
            };
        }

        // Universal commands (work in any mode)
        match key {
            Key::Esc | Key::Ctrl('c') => return Some(Message::Quit),
            Key::Char('\t') | Key::Char('\\') => return Some(Message::ModeToggle),
            _ => {}
        }

        // Handle Enter based on mode
        if let Key::Char('\n') = key {
            return match self.mode {
                Mode::Search => Some(Message::ModeToggle), // Enter toggles to Browse in Search mode
                Mode::Browse => Some(Message::OpenPaper(false)), // Enter opens paper in Browse mode
            };
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
            Key::Char('y') => Some(Message::YankBibtex),
            Key::Char('d') => Some(Message::DeletePaper),
            Key::Char('o') => Some(Message::OpenPaper(true)), // Alt open
            Key::Char('q') => Some(Message::Quit),            // Quick quit in browse mode
            // Future browse commands can be added here:
            // Key::Char(' ') => Some(Message::ToggleSelection),
            _ => None,
        }
    }

    // Update state based on message
    fn update(&mut self, msg: Message) -> Result<(), SearchError> {
        // Check if we should clear expired flash messages
        if let Some(MessageState::Flash { expires_at, .. }) = &self.message {
            if Instant::now() >= *expires_at {
                self.message = None;
            }
        }

        match msg {
            Message::ModeToggle => {
                self.mode = self.mode.toggle();
                if self.mode == Mode::Browse {
                    self.cursor_pos = 0;
                }
                self.message = None; // Clear any messages on mode switch
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
            Message::YankBibtex => {
                if let Some(paper) = self.get_selected_paper() {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if clipboard.set_text(&paper.bibtex).is_ok() {
                            self.message = Some(MessageState::Flash {
                                text: "BibTeX copied!".to_string(),
                                line_index: self.cursor_pos,
                                expires_at: Instant::now() + Duration::from_secs(2),
                            });
                        } else {
                            self.message = Some(MessageState::Flash {
                                text: "Failed to copy!".to_string(),
                                line_index: self.cursor_pos,
                                expires_at: Instant::now() + Duration::from_secs(2),
                            });
                        }
                    }
                }
            }
            Message::DeletePaper => {
                if self.get_selected_paper().is_some() {
                    self.message = Some(MessageState::Prompt {
                        text: "Delete this paper? [Y/n]".to_string(),
                        line_index: self.cursor_pos,
                        action: PendingAction::Delete(self.cursor_pos),
                    });
                }
            }
            Message::OpenPaper(alt_mode) => {
                if let Some(paper) = self.get_selected_paper() {
                    self.dummy_open_paper(paper, alt_mode);

                    let message_text = if alt_mode {
                        "Opening (alt)...".to_string()
                    } else {
                        "Opening...".to_string()
                    };

                    self.message = Some(MessageState::Flash {
                        text: message_text,
                        line_index: self.cursor_pos,
                        expires_at: Instant::now() + Duration::from_millis(1500),
                    });
                }
            }
            Message::ConfirmPrompt => {
                if let Some(MessageState::Prompt { action, .. }) = &self.message {
                    match action {
                        PendingAction::Delete(index) => {
                            // Get the paper to delete
                            if let Some(paper) = self.get_filtered_papers().get(*index) {
                                let paper_id = paper.id.clone();

                                // TODO: Actually delete from backend with something like:
                                // match self.store.delete(&paper_id) {
                                //     Ok(_) => {
                                //         self.papers.retain(|p| p.id != paper_id);
                                //         self.message = Some(MessageState::Flash {
                                //             text: "Paper deleted!".to_string(),
                                //             line_index: self.cursor_pos,
                                //             expires_at: Instant::now() + Duration::from_secs(2),
                                //         });
                                //     }
                                //     Err(e) => {
                                //         self.message = Some(MessageState::Flash {
                                //             text: format!("Failed: {}", e),
                                //             line_index: self.cursor_pos,
                                //             expires_at: Instant::now() + Duration::from_secs(3),
                                //         });
                                //     }
                                // }

                                // For now, dummy delete - just remove from local state
                                self.papers.retain(|p| p.id != paper_id);

                                // Adjust cursor if needed
                                let new_results_len = self.get_filtered_papers().len();
                                if self.cursor_pos >= new_results_len && new_results_len > 0 {
                                    self.cursor_pos = new_results_len - 1;
                                }

                                // Show confirmation
                                self.message = Some(MessageState::Flash {
                                    text: "Paper deleted!".to_string(),
                                    line_index: self.cursor_pos,
                                    expires_at: Instant::now() + Duration::from_secs(2),
                                });
                            }
                        }
                    }
                } else {
                    self.message = None;
                }
            }
            Message::CancelPrompt => {
                self.message = None;
            }
            Message::ClearMessage => {
                self.message = None;
            }
            Message::Quit => {} // Handled in run()
        }
        Ok(())
    }

    // Dummy open function for papers
    fn dummy_open_paper(&self, paper: &Paper, alt_mode: bool) {
        // TODO: Implement actual open functionality
        // For example:
        // if alt_mode {
        //     // Open in alternative viewer/browser
        //     open_in_browser(&paper.url);
        // } else {
        //     // Open in default PDF viewer
        //     open_pdf(&paper.path);
        // }

        // For now, just print to stderr so it doesn't mess up the TUI
        eprintln!("Opening paper '{}' (alt_mode: {})", paper.id, alt_mode);
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

            // Check if this line has a message to display
            let has_message = match &self.message {
                Some(MessageState::Flash {
                    line_index, text, ..
                })
                | Some(MessageState::Prompt {
                    line_index, text, ..
                }) => {
                    if *line_index == i {
                        // Display the message
                        if matches!(self.message, Some(MessageState::Prompt { .. })) {
                            // For prompts, replace the entire line
                            writeln!(self.stdout, "  {}\r", text)?;
                            true
                        } else {
                            // For flash messages, show with checkmark
                            writeln!(
                                self.stdout,
                                "\t✓ {}[{}]{}\r",
                                color::Fg(color::Green),
                                text,
                                color::Fg(color::Reset)
                            )?;
                            true
                        }
                    } else {
                        false
                    }
                }
                None => false,
            };

            if !has_message {
                // Normal display
                if self.mode == Mode::Browse && i == self.cursor_pos {
                    write!(self.stdout, "* ")?;
                } else {
                    write!(self.stdout, "  ")?;
                }
                write!(self.stdout, "{}", display)?;
                writeln!(self.stdout, "\r")?;
            }
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
            Mode::Search => "Enter/Tab: Browse | Esc: Quit | Type to search",
            Mode::Browse => match &self.message {
                Some(MessageState::Prompt { .. }) => "Y: Confirm | N/Esc: Cancel",
                _ => "Enter/o: Open | Tab: Search | j/k: Nav | y: Copy | d: Delete | q/Esc: Quit",
            },
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

    // Get currently selected paper
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
