use std::cmp::Ordering;
use sublime_fuzzy::{best_match, Match};

/// Represents an item that can be fuzzy searched
#[derive(Debug, Clone)]
pub struct SearchableItem {
    /// Primary text to search (e.g., title)
    pub primary: String,
    /// Secondary text to search (e.g., author)
    pub secondary: Option<String>,
    /// Additional context for display (e.g., year)
    pub context: Option<String>,
    /// Unique identifier
    pub id: String,
    /// Full display string for completion
    pub display: String,
}

/// Result of a fuzzy search with score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub item: SearchableItem,
    pub score: isize,
    pub primary_match: Option<Match>,
    pub secondary_match: Option<Match>,
}

impl SearchResult {
    /// Format for zsh completion display
    pub fn format_for_completion(&self, include_context: bool) -> String {
        if include_context {
            if let Some(ctx) = &self.item.context {
                format!("{}:{}", self.item.display, ctx)
            } else {
                self.item.display.clone()
            }
        } else {
            self.item.display.clone()
        }
    }

    /// Format just the value to insert (without description)
    pub fn completion_value(&self) -> String {
        // For titles with special characters, we might want to quote them
        if self
            .item
            .primary
            .contains(&[' ', '(', ')', '[', ']', '{', '}', '\'', '"'][..])
        {
            format!("\"{}\"", self.item.primary.replace("\"", "\\\""))
        } else {
            self.item.primary.clone()
        }
    }
}

/// Configuration for fuzzy search behavior
#[derive(Debug, Clone)]
pub struct FuzzyConfig {
    /// Minimum score to be considered a match (0-100)
    pub min_score: isize,
    /// Weight for primary field matches (e.g., title)
    pub primary_weight: f32,
    /// Weight for secondary field matches (e.g., author)
    pub secondary_weight: f32,
    /// Maximum number of results to return
    pub max_results: usize,
    /// Whether to search in secondary field
    pub search_secondary: bool,
}

impl Default for FuzzyConfig {
    fn default() -> Self {
        Self {
            min_score: 0, // Accept all matches, let sorting handle it
            primary_weight: 1.0,
            secondary_weight: 0.7,
            max_results: 50,
            search_secondary: true,
        }
    }
}

/// Main fuzzy search engine
pub struct FuzzySearcher {
    config: FuzzyConfig,
}

impl FuzzySearcher {
    pub fn new(config: FuzzyConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(FuzzyConfig::default())
    }

    /// Search through a list of items
    pub fn search(&self, query: &str, items: Vec<SearchableItem>) -> Vec<SearchResult> {
        if query.is_empty() {
            // Return all items with no score when query is empty
            return items
                .into_iter()
                .take(self.config.max_results)
                .map(|item| SearchResult {
                    item,
                    score: 0,
                    primary_match: None,
                    secondary_match: None,
                })
                .collect();
        }

        let mut results: Vec<SearchResult> = items
            .into_iter()
            .filter_map(|item| self.score_item(query, item))
            .filter(|result| result.score >= self.config.min_score)
            .collect();

        // Sort by score (highest first)
        results.sort_by(|a, b| b.score.cmp(&a.score));

        // Take only the configured maximum
        results.truncate(self.config.max_results);

        results
    }

    /// Score a single item against the query
    fn score_item(&self, query: &str, item: SearchableItem) -> Option<SearchResult> {
        let primary_match = best_match(query, &item.primary);
        let secondary_match = if self.config.search_secondary {
            item.secondary.as_ref().and_then(|s| best_match(query, s))
        } else {
            None
        };

        // Calculate combined score
        let primary_score = primary_match
            .as_ref()
            .map(|m| (m.score() as f32 * self.config.primary_weight) as isize)
            .unwrap_or(0);

        let secondary_score = secondary_match
            .as_ref()
            .map(|m| (m.score() as f32 * self.config.secondary_weight) as isize)
            .unwrap_or(0);

        let total_score = primary_score.max(secondary_score);

        if total_score > 0 {
            Some(SearchResult {
                item,
                score: total_score,
                primary_match,
                secondary_match,
            })
        } else {
            None
        }
    }

    /// Update configuration
    pub fn with_config(mut self, config: FuzzyConfig) -> Self {
        self.config = config;
        self
    }

    /// Set whether to search secondary field
    pub fn search_secondary(mut self, enabled: bool) -> Self {
        self.config.search_secondary = enabled;
        self
    }

    /// Set maximum results
    pub fn max_results(mut self, max: usize) -> Self {
        self.config.max_results = max;
        self
    }
}

/// Helper to format matches with highlighting (for future interactive mode)
pub fn format_with_highlight(text: &str, match_info: &Match) -> String {
    let mut result = String::new();
    let mut last_end = 0;

    for &index in match_info.matched_indices() {
        // Add text before match
        if index > last_end {
            result.push_str(&text[last_end..index]);
        }
        // Add highlighted character
        result.push_str(&format!("\x1b[1;33m{}\x1b[0m", &text[index..index + 1]));
        last_end = index + 1;
    }

    // Add remaining text
    if last_end < text.len() {
        result.push_str(&text[last_end..]);
    }

    result
}
