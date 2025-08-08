// src/completion.rs

use crate::base::Paper;
use crate::fuzzy::{FuzzyConfig, FuzzySearcher, SearchableItem};
use crate::store::{PaperStore, StoreError};
use std::io::{self, Write};

/// Different completion contexts
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionContext {
    /// Complete paper titles for search
    SearchTitle,
    /// Complete author names
    SearchAuthor,
    /// Complete paper keys (for delete, open, etc.)
    PaperKey,
    /// Complete years
    Year,
}

/// Completion handler
pub struct CompletionHandler<'a> {
    store: &'a PaperStore,
}

impl<'a> CompletionHandler<'a> {
    pub fn new(store: &'a PaperStore) -> Self {
        Self { store }
    }

    /// Generate completions for the given context and query
    pub fn complete(
        &self,
        context: CompletionContext,
        query: &str,
    ) -> Result<Vec<String>, StoreError> {
        match context {
            CompletionContext::SearchTitle => self.complete_titles(query),
            CompletionContext::SearchAuthor => self.complete_authors(query),
            CompletionContext::PaperKey => self.complete_keys(query),
            CompletionContext::Year => self.complete_years(query),
        }
    }

    /// Complete paper titles with fuzzy matching
    fn complete_titles(&self, query: &str) -> Result<Vec<String>, StoreError> {
        let papers = self.store.list_all(None)?;

        // Convert papers to searchable items
        let items: Vec<SearchableItem> = papers
            .into_iter()
            .map(|paper| SearchableItem {
                primary: paper.title.clone(),
                secondary: Some(paper.author.clone()),
                context: Some(format!("{} - {}", paper.author, paper.year)),
                id: paper.id.to_string(),
                display: paper.title,
            })
            .collect();

        // Perform fuzzy search
        let config = FuzzyConfig {
            max_results: 20,
            search_secondary: true,
            ..Default::default()
        };

        let searcher = FuzzySearcher::new(config);
        let results = searcher.search(query, items);

        // Format for zsh completion
        Ok(results
            .into_iter()
            .map(|r| {
                format!(
                    "{}:{} - {}",
                    r.completion_value(),
                    r.item.context.unwrap_or_default(),
                    r.item.id
                )
            })
            .collect())
    }

    /// Complete author names with fuzzy matching
    fn complete_authors(&self, query: &str) -> Result<Vec<String>, StoreError> {
        let papers = self.store.list_all(None)?;

        // Get unique authors (note: these are already abbreviated like "Smith et al.")
        let mut authors: Vec<String> = papers.into_iter().map(|p| p.author).collect();

        // Remove duplicates
        authors.sort();
        authors.dedup();

        // Convert to searchable items
        let items: Vec<SearchableItem> = authors
            .into_iter()
            .map(|author| SearchableItem {
                primary: author.clone(),
                secondary: None,
                context: None,
                id: author.clone(),
                display: author,
            })
            .collect();

        // Perform fuzzy search
        let config = FuzzyConfig {
            max_results: 15,
            search_secondary: false,
            ..Default::default()
        };

        let searcher = FuzzySearcher::new(config);
        let results = searcher.search(query, items);

        // Return just the author names for completion
        Ok(results.into_iter().map(|r| r.item.primary).collect())
    }

    /// Complete paper keys with fuzzy matching
    fn complete_keys(&self, query: &str) -> Result<Vec<String>, StoreError> {
        let papers = self.store.list_all(None)?;

        // Convert to searchable items with title as context
        let items: Vec<SearchableItem> = papers
            .into_iter()
            .map(|paper| SearchableItem {
                primary: paper.key.clone(),
                secondary: Some(paper.title.clone()),
                context: Some(format!("{} ({})", paper.title, paper.year)),
                id: paper.id.to_string(),
                display: paper.key,
            })
            .collect();

        // Perform fuzzy search on both key and title
        let config = FuzzyConfig {
            max_results: 15,
            search_secondary: true,
            ..Default::default()
        };

        let searcher = FuzzySearcher::new(config);
        let results = searcher.search(query, items);

        // Format: key:description
        Ok(results
            .into_iter()
            .map(|r| format!("{}:{}", r.item.primary, r.item.context.unwrap_or_default()))
            .collect())
    }

    /// Complete years (simple prefix matching since years are numbers)
    fn complete_years(&self, query: &str) -> Result<Vec<String>, StoreError> {
        let papers = self.store.list_all(None)?;

        // Get unique years
        let mut years: Vec<i64> = papers.into_iter().map(|p| p.year).collect();

        years.sort_by(|a, b| b.cmp(a)); // Sort descending (newest first)
        years.dedup();

        // Simple prefix matching for years
        let matching_years: Vec<String> = years
            .into_iter()
            .map(|y| y.to_string())
            .filter(|y| y.starts_with(query))
            .take(10)
            .collect();

        Ok(matching_years)
    }
}

/// Output completions in zsh format
pub fn output_completions_zsh(completions: Vec<String>) {
    for completion in completions {
        println!("{}", completion);
    }
}

/// Output completions in bash format
pub fn output_completions_bash(completions: Vec<String>) {
    for completion in completions {
        // For bash, we typically just output the completion value without description
        let value = completion.split(':').next().unwrap_or(&completion);
        println!("{}", value);
    }
}

/// Output completions in fish format
pub fn output_completions_fish(completions: Vec<String>) {
    for completion in completions {
        if let Some((value, description)) = completion.split_once(':') {
            println!("{}\t{}", value, description);
        } else {
            println!("{}", completion);
        }
    }
}
