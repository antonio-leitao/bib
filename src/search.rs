use crate::config::Config;
use crate::database::{CitationDb, DbError, Paper};
use crate::ui::StatusUI;
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
    #[error("UI error: {0}")]
    Ui(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] DbError),

    #[error("No results found matching the query")]
    NoResults,
}

pub async fn run(cfg: &Config, db: &mut CitationDb, limit: usize, sources_only: bool) -> Result<(), SearchError> {
    let ui = SearchUI::init(cfg, db, limit, sources_only).await?;
    ui.run()
}

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
    Flash { text: String, expires_at: Instant },
}

enum Message {
    ModeToggle,
    SearchInput(char),
    SearchBackspace,
    BrowseUp,
    BrowseDown,
    PullPdf,
    OpenPaper,
    Quit,
}

struct SearchUI<'a> {
    papers: Vec<Paper>,
    query: String,
    mode: Mode,
    cursor_pos: usize,
    limit: usize,
    stdout: RawTerminal<Stdout>,
    db: &'a mut CitationDb,
    cfg: &'a Config,
    message: Option<MessageState>,
}

impl<'a> SearchUI<'a> {
    async fn init(
        cfg: &'a Config,
        db: &'a mut CitationDb,
        limit: usize,
        sources_only: bool,
    ) -> Result<Self, SearchError> {
        let papers = db.get_papers(&[], sources_only)?;
        let mut stdout = io::stdout().into_raw_mode()?;
        write!(stdout, "{}", termion::cursor::Hide)?;

        let mut ui = SearchUI {
            papers,
            query: String::new(),
            mode: Mode::Search,
            cursor_pos: 0,
            limit,
            stdout,
            db,
            cfg,
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
                Mode::Browse => Some(Message::OpenPaper),  // Enter opens paper in Browse mode
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
            Key::Char('p') => Some(Message::PullPdf),
            Key::Char('q') => Some(Message::Quit), // Quick quit in browse mode
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
            Message::PullPdf => {
                if let Some(paper) = self.get_selected_paper() {
                    let pdf_path = self.cfg.pdf_dir().join(format!("{}.pdf", paper.key));

                    if !pdf_path.exists() {
                        self.message = Some(MessageState::Flash {
                            text: format!("{} PDF not found for: {}", StatusUI::ERROR, paper.key),
                            expires_at: Instant::now() + Duration::from_secs(3),
                        });
                    } else {
                        let dest_path = std::env::current_dir()
                            .unwrap_or_else(|_| std::path::PathBuf::from("."))
                            .join(format!("{}.pdf", paper.key));

                        match std::fs::copy(&pdf_path, &dest_path) {
                            Ok(_) => {
                                self.db.touch_paper(&paper.key)?;
                                self.message = Some(MessageState::Flash {
                                    text: format!("{} Pulled: {}", StatusUI::SUCCESS, paper.key),
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
            Message::OpenPaper => {
                if let Some(paper) = self.get_selected_paper() {
                    let url_to_open = if paper.processed {
                        let pdf_path = self.cfg.pdf_dir().join(format!("{}.pdf", paper.key));
                        Some(format!("file://{}", pdf_path.canonicalize()?.display()))
                    } else {
                        paper.link.clone()
                    };

                    match url_to_open {
                        Some(url) => match webbrowser::open(&url) {
                            Ok(()) => {
                                self.db.touch_paper(&paper.key)?;
                            }
                            Err(e) => {
                                self.message = Some(MessageState::Flash {
                                    text: format!("{} Failed to open: {}", StatusUI::ERROR, e),
                                    expires_at: Instant::now() + Duration::from_secs(3),
                                });
                            }
                        },
                        None => {
                            self.message = Some(MessageState::Flash {
                                text: format!("{} No URL available", StatusUI::ERROR),
                                expires_at: Instant::now() + Duration::from_secs(3),
                            });
                        }
                    }
                }
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
            .map(|paper| paper.display(width.saturating_sub(4) as usize).with_tags().to_string())
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
            "{}{}{} > {}{}\r",
            termion::clear::CurrentLine,
            color::Fg(color::Rgb(83, 110, 122)),
            mode_indicator,
            color::Fg(color::Reset),
            self.query
        )?;

        // Print results with cursor in browse mode
        for (i, display) in result_displays.iter().enumerate() {
            write!(self.stdout, "{}", termion::clear::CurrentLine)?;

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
                    "  {} {}",
                    self.format_command("Enter/Tab", "Browse"),
                    self.format_command("Esc", "Quit"),
                );
                writeln!(self.stdout, "{}\r", row)?;
                // Empty second row
                writeln!(self.stdout, "{}\r", termion::clear::CurrentLine)?;
            }
            Mode::Browse => {
                // First row with 4 columns
                let row1 = format!(
                    "  {} {} {}",
                    self.format_command("Enter", "Open"),
                    self.format_command("Tab", "Search"),
                    self.format_command("j/k", "Navigate"),
                );
                writeln!(self.stdout, "{}\r", row1)?;

                // Second row with 4 columns
                write!(self.stdout, "{}", termion::clear::CurrentLine)?;
                let row2 = format!(
                    "  {} {}",
                    self.format_command("p", "Pull PDF"),
                    self.format_command("q/Esc", "Quit"),
                );
                writeln!(self.stdout, "{}\r", row2)?;
            }
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
    fn format_command(&self, key: &str, desc: &str) -> String {
        format!(
            "{}{:>7} • {}{}{:<8}{}",
            color::Fg(color::Rgb(83, 110, 122)),
            key,
            color::Fg(color::Reset),
            color::Fg(color::Rgb(46, 60, 68)),
            desc,
            color::Fg(color::Reset)
        )
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
                .filter(|paper| {
                    best_match(
                        &self.query,
                        &format!(
                            "{}{}",
                            paper.authors.clone().unwrap_or(String::new()),
                            paper.title.clone().unwrap_or(String::new())
                        ),
                    )
                    .is_some()
                })
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
            best_match(
                query,
                &format!(
                    "{}{}",
                    paper.authors.clone().unwrap_or(String::new()),
                    paper.title.clone().unwrap_or(String::new())
                ),
            )
            .map(|fuzzy_match| (fuzzy_match.score(), paper))
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
