mod error;
pub use error::SearchError;

use crate::core::Paper;
use crate::pdf::PdfStorage;
use crate::storage::PaperStore;
use crate::ui::StatusUI;
use std::io::{self, Stdout, Write};
use std::time::{Duration, Instant};
use sublime_fuzzy::best_match;
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

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

enum MessageState {
    Flash {
        text: String,
        expires_at: Instant,
    },
    Prompt {
        text: String,
        line_index: usize,
        action: PendingAction,
    },
}

enum PendingAction {
    Delete(usize),
}

enum Message {
    ModeToggle,
    SearchInput(char),
    SearchBackspace,
    BrowseUp,
    BrowseDown,
    YankBibtex,
    DeletePaper,
    PullPdf,
    OpenPaper(bool),
    ConfirmPrompt,
    CancelPrompt,
    ClearMessage,
    Quit,
}

struct SearchUI<'a> {
    papers: Vec<Paper>,
    query: String,
    mode: Mode,
    cursor_pos: usize,
    limit: usize,
    stdout: RawTerminal<Stdout>,
    store: &'a mut PaperStore,
    message: Option<MessageState>,
}

impl<'a> SearchUI<'a> {
    fn init(store: &'a mut PaperStore, limit: usize) -> Result<Self, SearchError> {
        let papers = store.list_all(None)?;
        let mut stdout = io::stdout().into_raw_mode()?;
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

        ui.view()?;
        Ok(ui)
    }

    fn run(mut self) -> Result<(), SearchError> {
        let stdin = io::stdin();

        for key in stdin.keys() {
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
                None => {}
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
            Key::Char('p') => Some(Message::PullPdf),
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
                            self.store.touch(paper.id)?;
                            self.message = Some(MessageState::Flash {
                                text: format!("{} BibTeX copied!", StatusUI::SUCCESS),
                                expires_at: Instant::now() + Duration::from_secs(2),
                            });
                        } else {
                            self.message = Some(MessageState::Flash {
                                text: format!("{} Failed to copy!", StatusUI::ERROR),
                                expires_at: Instant::now() + Duration::from_secs(2),
                            });
                        }
                    }
                }
            }
            Message::PullPdf => {
                if let Some(paper) = self.get_selected_paper() {
                    let source_path = paper.pdf_path();

                    if !source_path.exists() {
                        self.message = Some(MessageState::Flash {
                            text: format!("{} PDF not found for: {}", StatusUI::ERROR, paper.key),
                            expires_at: Instant::now() + Duration::from_secs(3),
                        });
                    } else {
                        let dest_filename = format!(
                            "{}_{}.pdf",
                            paper.key,
                            &paper.id.to_string()[..5.min(paper.id.to_string().len())]
                        );
                        let dest_path = std::env::current_dir()
                            .unwrap_or_else(|_| std::path::PathBuf::from("."))
                            .join(&dest_filename);

                        match std::fs::copy(&source_path, &dest_path) {
                            Ok(_) => {
                                self.message = Some(MessageState::Flash {
                                    text: format!(
                                        "{} Pulled: {}",
                                        StatusUI::SUCCESS,
                                        dest_filename
                                    ),
                                    expires_at: Instant::now() + Duration::from_secs(2),
                                });
                            }
                            Err(e) => {
                                self.message = Some(MessageState::Flash {
                                    text: format!("{} Pull failed: {}", StatusUI::ERROR, e),
                                    expires_at: Instant::now() + Duration::from_secs(3),
                                });
                            }
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
                    match paper.open_pdf(alt_mode) {
                        Ok(()) => self.store.touch(paper.id)?,
                        Err(e) => {
                            self.message = Some(MessageState::Flash {
                                text: format!("{} Failed: {}", StatusUI::ERROR, e),
                                expires_at: Instant::now() + Duration::from_secs(3),
                            });
                        }
                    }
                }
            }
            Message::ConfirmPrompt => {
                if let Some(MessageState::Prompt { action, .. }) = &self.message {
                    match action {
                        PendingAction::Delete(index) => {
                            // Get the paper to delete
                            if let Some(paper) = self.get_filtered_papers().get(*index) {
                                let paper_id = paper.id;
                                let paper_key = paper.key.clone();

                                // First try to delete the PDF file
                                let pdf_deleted = if paper.pdf_exists() {
                                    match PdfStorage::delete_pdf(paper) {
                                        Ok(_) => true,
                                        Err(e) => {
                                            // Log PDF deletion failure but continue
                                            eprintln!(
                                                "Warning: Failed to delete PDF for {}: {}",
                                                paper_key, e
                                            );
                                            false
                                        }
                                    }
                                } else {
                                    true // No PDF to delete
                                };

                                // Now delete from database
                                match self.store.delete(paper_id) {
                                    Ok(_) => {
                                        // Remove from local state
                                        self.papers.retain(|p| p.id != paper_id);

                                        // Adjust cursor if needed
                                        let new_results_len = self.get_filtered_papers().len();
                                        if self.cursor_pos >= new_results_len && new_results_len > 0
                                        {
                                            self.cursor_pos = new_results_len - 1;
                                        }

                                        // Show appropriate confirmation message
                                        let message_text = if pdf_deleted {
                                            format!("{} Deleted: {}", StatusUI::SUCCESS, paper_key)
                                        } else {
                                            format!(
                                                "{} Deleted: {} (PDF removal failed)",
                                                StatusUI::WARNING,
                                                paper_key
                                            )
                                        };

                                        self.message = Some(MessageState::Flash {
                                            text: message_text,
                                            expires_at: Instant::now() + Duration::from_secs(2),
                                        });
                                    }
                                    Err(e) => {
                                        self.message = Some(MessageState::Flash {
                                            text: format!(
                                                "{} Delete failed: {}",
                                                StatusUI::ERROR,
                                                e
                                            ),
                                            expires_at: Instant::now() + Duration::from_secs(3),
                                        });
                                    }
                                }
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
            "{}{}{}{} > {}\r",
            termion::clear::CurrentLine,
            color::Fg(color::Rgb(83, 110, 122)),
            mode_indicator,
            color::Fg(color::Reset),
            self.query
        )?;

        // Print results with cursor in browse mode
        for (i, display) in result_displays.iter().enumerate() {
            write!(self.stdout, "{}", termion::clear::CurrentLine)?;

            // Check if this line has a PROMPT message to display (prompts stay inline)
            let has_prompt = match &self.message {
                Some(MessageState::Prompt {
                    line_index, text, ..
                }) => {
                    if *line_index == i {
                        // Display the prompt inline
                        writeln!(self.stdout, "  {}\r", text)?;
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if !has_prompt {
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

        // Buffer line: Show flash message OR hidden count
        write!(self.stdout, "{}", termion::clear::CurrentLine)?;
        match &self.message {
            Some(MessageState::Flash { text, .. }) => {
                // Show flash message in the buffer line
                writeln!(self.stdout, "  {}\r", text)?;
            }
            _ => {
                // Show hidden results count if applicable
                if hidden_count > 0 {
                    writeln!(
                        self.stdout,
                        "{}  ... {} more results hidden{}\r",
                        color::Fg(color::Rgb(46, 60, 68)),
                        hidden_count,
                        color::Fg(color::Reset)
                    )?;
                } else {
                    writeln!(self.stdout, "\r")?; // Empty line
                }
            }
        }

        // Show commands based on mode (2-row format)
        write!(self.stdout, "{}", termion::clear::CurrentLine)?;
        match self.mode {
            Mode::Search => {
                // Search mode: only one row of help
                let row = format!(
                    "  {} {} {}",
                    self.format_command("Enter/Tab: Browse"),
                    self.format_command("Esc: Quit"),
                    self.format_command("Type to search")
                );
                writeln!(self.stdout, "{}\r", row)?;
                // Empty second row
                writeln!(self.stdout, "{}\r", termion::clear::CurrentLine)?;
            }
            Mode::Browse => match &self.message {
                Some(MessageState::Prompt { .. }) => {
                    writeln!(
                        self.stdout,
                        "  {}{:>9} • {}{}{:<9}{} {}{:>9} • {}{}{:<9}{}\r",
                        color::Fg(color::Rgb(83, 110, 122)),
                        "Y",
                        color::Fg(color::Reset),
                        color::Fg(color::Rgb(46, 60, 68)),
                        "Confirm",
                        color::Fg(color::Reset),
                        color::Fg(color::Rgb(83, 110, 122)),
                        "N/Esc",
                        color::Fg(color::Reset),
                        color::Fg(color::Rgb(46, 60, 68)),
                        "Cancel",
                        color::Fg(color::Reset)
                    )?;
                    writeln!(self.stdout, "{}\r", termion::clear::CurrentLine)?;
                }
                _ => {
                    // First row with 4 columns
                    let row1 = format!(
                        "  {} {} {} {}",
                        self.format_command("Enter/o: Open"),
                        self.format_command("Tab: Search"),
                        self.format_command("j/k: Navigate"),
                        self.format_command("y: Copy BibTeX")
                    );
                    writeln!(self.stdout, "{}\r", row1)?;

                    // Second row with 4 columns
                    write!(self.stdout, "{}", termion::clear::CurrentLine)?;
                    let row2 = format!(
                        "  {} {} {} {}",
                        self.format_command("d: Delete"),
                        self.format_command("p: Pull PDF"),
                        self.format_command("q/Esc: Quit"),
                        self.format_command("")
                    );
                    writeln!(self.stdout, "{}\r", row2)?;
                }
            },
        }

        // Move cursor back up (now accounting for 4 lines total: header + papers + buffer + 2 help rows)
        write!(
            self.stdout,
            "{}",
            termion::cursor::Up(self.limit as u16 + 4)
        )?;
        self.stdout.flush()?;
        Ok(())
    }
    // Helper function to format command help
    fn format_command(&self, cmd: &str) -> String {
        if let Some(colon_pos) = cmd.find(':') {
            let key = cmd[..colon_pos].trim();
            let desc = cmd[colon_pos + 1..].trim();

            format!(
                "{}{:>9} • {}{}{:<9}{}",
                color::Fg(color::Rgb(83, 110, 122)),
                key,
                color::Fg(color::Reset),
                color::Fg(color::Rgb(46, 60, 68)),
                desc,
                color::Fg(color::Reset)
            )
        } else {
            // No colon, treat as description only
            format!(
                "{:>9}{}{}{}",
                "",
                color::Fg(color::Rgb(46, 60, 68)),
                cmd,
                color::Fg(color::Reset)
            )
        }
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

pub fn execute(store: &mut PaperStore, limit: usize) -> Result<(), SearchError> {
    let ui = SearchUI::init(store, limit)?;
    ui.run()
}
