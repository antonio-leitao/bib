mod error;
pub use error::FindError;

use crate::ai::Gemini;
use crate::core::{Embedding, Paper};
use crate::pdf::PdfStorage;
use crate::storage::PaperStore;
use crate::ui::StatusUI;
use dotzilla;
use std::cmp::Ordering;
use std::io::{self, Stdout, Write};
use std::time::{Duration, Instant};
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

struct ScoredPaper {
    paper: Paper,
    score: f32,
}

struct SemanticSearchUI<'a> {
    papers: Vec<ScoredPaper>,
    query: String,
    cursor_pos: usize,
    limit: usize,
    stdout: RawTerminal<Stdout>,
    store: &'a mut PaperStore,
    message: Option<MessageState>,
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
    BrowseUp,
    BrowseDown,
    YankBibtex,
    DeletePaper,
    PullPdf,
    OpenPaper(bool),
    ConfirmPrompt,
    CancelPrompt,
    Quit,
}

impl<'a> SemanticSearchUI<'a> {
    fn init(
        store: &'a mut PaperStore,
        papers: Vec<ScoredPaper>,
        query: String,
        limit: usize,
    ) -> Result<Self, FindError> {
        let mut stdout = io::stdout().into_raw_mode()?;
        write!(stdout, "{}", termion::cursor::Hide)?;

        let mut ui = SemanticSearchUI {
            papers,
            query,
            cursor_pos: 0,
            limit,
            stdout,
            store,
            message: None,
        };

        ui.view()?;
        Ok(ui)
    }

    fn run(mut self) -> Result<(), FindError> {
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

        // Browse mode commands
        match key {
            Key::Char('j') | Key::Down => Some(Message::BrowseDown),
            Key::Char('k') | Key::Up => Some(Message::BrowseUp),
            Key::Char('y') => Some(Message::YankBibtex),
            Key::Char('d') => Some(Message::DeletePaper),
            Key::Char('p') => Some(Message::PullPdf),
            Key::Char('\n') => Some(Message::OpenPaper(false)),
            Key::Char('o') => Some(Message::OpenPaper(true)), // Alt open
            Key::Char('q') | Key::Esc | Key::Ctrl('c') => Some(Message::Quit),
            _ => None,
        }
    }

    // Update state based on message
    fn update(&mut self, msg: Message) -> Result<(), FindError> {
        // Check if we should clear expired flash messages
        if let Some(MessageState::Flash { expires_at, .. }) = &self.message {
            if Instant::now() >= *expires_at {
                self.message = None;
            }
        }

        match msg {
            Message::BrowseUp => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            Message::BrowseDown => {
                let max_pos = self.papers.len().min(self.limit).saturating_sub(1);
                if self.cursor_pos < max_pos {
                    self.cursor_pos += 1;
                }
            }
            Message::YankBibtex => {
                if let Some(scored_paper) = self.get_selected_paper() {
                    let paper = &scored_paper.paper;
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
                if let Some(scored_paper) = self.get_selected_paper() {
                    let paper = &scored_paper.paper;
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
                if let Some(scored_paper) = self.get_selected_paper() {
                    let paper = &scored_paper.paper;
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
                            if let Some(scored_paper) = self.papers.get(*index) {
                                let paper = &scored_paper.paper;
                                let paper_id = paper.id;
                                let paper_key = paper.key.clone();

                                // First try to delete the PDF file
                                let pdf_deleted = if paper.pdf_exists() {
                                    match PdfStorage::delete_pdf(paper) {
                                        Ok(_) => true,
                                        Err(e) => {
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
                                        self.papers.retain(|sp| sp.paper.id != paper_id);

                                        // Adjust cursor if needed
                                        let new_results_len = self.papers.len().min(self.limit);
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
            Message::Quit => {} // Handled in run()
        }
        Ok(())
    }

    // Helper function to format command help
    fn format_command(&self, cmd: &str) -> String {
        if let Some(colon_pos) = cmd.find(':') {
            let key = cmd[..colon_pos].trim();
            let desc = cmd[colon_pos + 1..].trim();

            format!(
                "{}{:>7} • {}{}{:<7}{}",
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
                "{:>7}{}{}{}",
                "",
                color::Fg(color::Rgb(46, 60, 68)),
                cmd,
                color::Fg(color::Reset)
            )
        }
    }

    // Updated view method - replace the help section
    fn view(&mut self) -> Result<(), FindError> {
        let (width, _) = termion::terminal_size()?;
        let display_limit = self.papers.len().min(self.limit);
        let hidden_count = self.papers.len().saturating_sub(self.limit);

        // Show header with query
        writeln!(
            self.stdout,
            "{}{}[SEMANTIC SEARCH] \"{}\"{}\r",
            termion::clear::CurrentLine,
            color::Fg(color::Rgb(83, 110, 122)),
            self.query,
            color::Fg(color::Reset)
        )?;

        // Print results with cursor
        for i in 0..display_limit {
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
                if i == self.cursor_pos {
                    write!(self.stdout, "* ")?;
                } else {
                    write!(self.stdout, "  ")?;
                }

                // Display paper with similarity score
                if let Some(scored_paper) = self.papers.get(i) {
                    let display = scored_paper.paper.display(width.saturating_sub(15));
                    writeln!(self.stdout, "{}\r", display)?;
                } else {
                    writeln!(self.stdout, "\r")?;
                }
            }
        }

        // Print blank lines for remaining slots
        for i in display_limit..self.limit {
            write!(self.stdout, "{}", termion::clear::CurrentLine)?;

            if i == self.cursor_pos {
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

        // Show commands in 2-row format
        write!(self.stdout, "{}", termion::clear::CurrentLine)?;
        match &self.message {
            Some(MessageState::Prompt { .. }) => {
                writeln!(
                    self.stdout,
                    "  {}{:>7} • {}{}{:<7}{} {}{:>7} • {}{}{:<7}{}\r",
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

    // Get currently selected paper
    fn get_selected_paper(&self) -> Option<&ScoredPaper> {
        self.papers.get(self.cursor_pos)
    }

    // Cleanup when exiting
    fn cleanup(&mut self) -> Result<(), FindError> {
        write!(
            self.stdout,
            "{}{}",
            termion::clear::AfterCursor,
            termion::cursor::Show
        )?;
        Ok(())
    }
}

fn similarity_threshold_filter(
    vectors: Vec<Embedding>,
    query: &[f32],
    k: usize,
    threshold: f32,
) -> Vec<(u128, f32)> {
    if vectors.is_empty() || k == 0 {
        return Vec::new();
    }

    let mut scores: Vec<_> = vectors
        .into_iter()
        .map(|e| (e.id, dotzilla::dot(query, &e.coords)))
        .collect();

    let k = k.min(scores.len());

    scores.select_nth_unstable_by(k - 1, |a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
    });

    scores[..k].sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    scores.truncate(k);

    scores
        .into_iter()
        .filter(|(_, sim)| *sim >= threshold)
        .collect()
}

pub async fn execute(
    store: &mut PaperStore,
    query: &str,
    limit: usize,
    threshold: f32,
) -> Result<(), FindError> {
    let spinner = StatusUI::spinner("Generating query embedding...");
    let ai = Gemini::new()?;
    let query_vector = ai.generate_query_embedding(query).await?;
    StatusUI::finish_spinner_success(spinner, "Generated query embedding");

    let spinner = StatusUI::spinner("Searching for similar papers...");
    let vectors = store.load_all_embeddings()?;
    let relevant_scores = similarity_threshold_filter(vectors, &query_vector, limit, threshold);

    if relevant_scores.is_empty() {
        StatusUI::finish_spinner_warning(
            spinner,
            &format!("No papers found above threshold {:.2}", threshold),
        );
        return Err(FindError::NoResults);
    }

    StatusUI::finish_spinner_success(
        spinner,
        &format!("Found {} relevant papers", relevant_scores.len()),
    );

    let id_list: Vec<u128> = relevant_scores.iter().map(|(id, _)| *id).collect();
    let papers = store.get_by_ids(&id_list)?;

    let mut scored_papers = Vec::new();
    for (id, score) in relevant_scores {
        if let Some(paper) = papers.iter().find(|p| p.id == id) {
            scored_papers.push(ScoredPaper {
                paper: paper.clone(),
                score,
            });
        }
    }

    let ui = SemanticSearchUI::init(store, scored_papers, query.to_string(), limit)?;
    ui.run()?;

    Ok(())
}
