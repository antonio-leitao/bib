use crate::base::{Embedding, Paper, PdfError, PdfStorage, UI};
use crate::blog_warning;
use crate::gemini::Gemini;
use crate::store::{PaperStore, StoreError};
use anyhow::Result;
use dotzilla;
use std::cmp::Ordering;
use std::io::{self, Stdout, Write};
use std::time::{Duration, Instant};
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SemanticSearchError {
    #[error("Store error: {0}")]
    Store(#[from] StoreError),
    #[error("UI error: {0}")]
    IOError(#[from] io::Error),
    #[error("PDF error: {0}")]
    Pdf(#[from] PdfError),
    #[error("Gemini error: {0}")]
    Gemini(#[from] crate::gemini::GeminiError),
    #[error("No results found")]
    NoResults,
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
    Delete(usize), // paper index in results
}

// Message enum for updates
enum Message {
    BrowseUp,
    BrowseDown,
    YankBibtex,
    DeletePaper,
    OpenPaper(bool), // bool flag for alt mode
    ConfirmPrompt,
    CancelPrompt,
    Quit,
}

// Paper with similarity score
struct ScoredPaper {
    paper: Paper,
    score: f32,
}

// Main UI struct for semantic search
struct SemanticSearchUI<'a> {
    papers: Vec<ScoredPaper>,
    query: String,
    cursor_pos: usize,
    limit: usize,
    stdout: RawTerminal<Stdout>,
    store: &'a mut PaperStore,
    message: Option<MessageState>,
}

impl<'a> SemanticSearchUI<'a> {
    // Initialize the UI with pre-ranked papers
    fn init(
        store: &'a mut PaperStore,
        papers: Vec<ScoredPaper>,
        query: String,
        limit: usize,
    ) -> Result<Self, SemanticSearchError> {
        let mut stdout = io::stdout().into_raw_mode()?;

        // Hide cursor
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

        // Initial render
        ui.view()?;

        Ok(ui)
    }

    // Main event loop
    fn run(mut self) -> Result<(), SemanticSearchError> {
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

        // Browse mode commands
        match key {
            Key::Char('j') | Key::Down => Some(Message::BrowseDown),
            Key::Char('k') | Key::Up => Some(Message::BrowseUp),
            Key::Char('y') => Some(Message::YankBibtex),
            Key::Char('d') => Some(Message::DeletePaper),
            Key::Char('\n') => Some(Message::OpenPaper(false)),
            Key::Char('o') => Some(Message::OpenPaper(true)), // Alt open
            Key::Char('q') | Key::Esc | Key::Ctrl('c') => Some(Message::Quit),
            _ => None,
        }
    }

    // Update state based on message
    fn update(&mut self, msg: Message) -> Result<(), SemanticSearchError> {
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
                if let Some(scored_paper) = self.get_selected_paper() {
                    let paper = &scored_paper.paper;
                    match paper.open_pdf(alt_mode) {
                        Ok(()) => self.store.touch(paper.id)?,
                        Err(e) => {
                            self.message = Some(MessageState::Flash {
                                text: format!("Failed: {}", e),
                                line_index: self.cursor_pos,
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
                                            format!("Deleted: {}", paper_key)
                                        } else {
                                            format!("Deleted: {} (PDF removal failed)", paper_key)
                                        };

                                        self.message = Some(MessageState::Flash {
                                            text: message_text,
                                            line_index: self.cursor_pos,
                                            expires_at: Instant::now() + Duration::from_secs(2),
                                        });
                                    }
                                    Err(e) => {
                                        self.message = Some(MessageState::Flash {
                                            text: format!("Delete failed: {}", e),
                                            line_index: self.cursor_pos,
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

    // Render the UI
    fn view(&mut self) -> Result<(), SemanticSearchError> {
        let (width, _) = termion::terminal_size()?;
        let display_limit = self.papers.len().min(self.limit);
        let hidden_count = self.papers.len().saturating_sub(self.limit);

        // Show header with query
        writeln!(
            self.stdout,
            "{}{}[SEMANTIC SEARCH] Query: \"{}\"{}\r",
            termion::clear::CurrentLine,
            color::Fg(color::Rgb(83, 110, 122)),
            self.query,
            color::Fg(color::Reset)
        )?;

        // Print results with cursor
        for i in 0..display_limit {
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

        // Show commands
        write!(self.stdout, "{}", termion::clear::CurrentLine)?;
        let commands = match &self.message {
            Some(MessageState::Prompt { .. }) => "Y: Confirm | N/Esc: Cancel",
            _ => "Enter/o: Open | j/k: Navigate | y: Copy BibTeX | d: Delete | q/Esc: Quit",
        };
        writeln!(self.stdout, "  {}\r", commands)?;

        // Move cursor back up
        write!(
            self.stdout,
            "{}",
            termion::cursor::Up(self.limit as u16 + 3)
        )?;
        self.stdout.flush()?;
        Ok(())
    }

    // Get currently selected paper
    fn get_selected_paper(&self) -> Option<&ScoredPaper> {
        self.papers.get(self.cursor_pos)
    }

    // Cleanup when exiting
    fn cleanup(&mut self) -> Result<(), SemanticSearchError> {
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

    // Partition so that top k elements are at the beginning
    scores.select_nth_unstable_by(k - 1, |a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
    });

    // Sort only the top k elements
    scores[..k].sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    scores.truncate(k);

    scores
        .into_iter()
        .filter(|(_, sim)| *sim >= threshold)
        .collect()
}

// Main entry point for semantic search
pub async fn find(
    store: &mut PaperStore,
    query: &str,
    limit: usize,
    threshold: f32,
) -> Result<(), SemanticSearchError> {
    // Generate query embedding
    let spinner = UI::spinner("Generating", "Query embedding...");
    let ai = Gemini::new()?;
    let query_vector = ai.generate_query_embedding(query).await?;
    UI::finish_with_message(spinner, "Generated", "query embedding.");

    // Search for similar papers
    let spinner = UI::spinner("Searching", "for similar papers...");
    let vectors = store.load_all_embeddings()?;
    let relevant_scores = similarity_threshold_filter(vectors, &query_vector, limit, threshold); // Get all scores
    match relevant_scores.is_empty() {
        true => {
            blog_warning!(
                "No results",
                "No papers found above threshold {:.2}",
                threshold
            );
            return Err(SemanticSearchError::NoResults);
        }
        false => UI::finish_with_message(
            spinner,
            "Found",
            &format!("{} relevant papers.", relevant_scores.len()),
        ),
    }

    // Get paper details for the filtered results
    let id_list: Vec<u128> = relevant_scores.iter().map(|(id, _)| *id).collect();
    let papers = store.get_by_ids(&id_list)?;

    // Create scored papers list, maintaining the order from k_nearest
    let mut scored_papers = Vec::new();
    for (id, score) in relevant_scores {
        if let Some(paper) = papers.iter().find(|p| p.id == id) {
            scored_papers.push(ScoredPaper {
                paper: paper.clone(),
                score,
            });
        }
    }

    // Initialize and run the interactive UI
    let ui = SemanticSearchUI::init(store, scored_papers, query.to_string(), limit)?;
    ui.run()?;

    Ok(())
}
