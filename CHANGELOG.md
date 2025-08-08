# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.11] - 2025-08-08

### Added
- Initial release of bib paper manager
- Add papers from arXiv URLs, PDF URLs, or local files
- Automatic BibTeX extraction using AI (Gemini)
- Fuzzy search for papers by title or author
- SQLite database for paper storage
- Content-based deduplication using SHA-256 hashing
- Shell completions for zsh, bash, and fish
- Automatic DOI lookup for arXiv papers
- Progress indicators for downloads and operations
- Database statistics command

### Features
- Smart paper identification from clipboard
- Automatic upgrade to official BibTeX when DOI is available
- Abbreviated author display (e.g., "Smith et al.")
- Fuzzy matching using sublime_fuzzy algorithm
- Native shell completion integration

