<p align="center">
  <img src='assets/logo.svg' width='250px' align="center"></img>
</p>

<div align="center">
<h3 max-width='200px' align="center"><code>bib</code></h3>
  <p><i>Manage your entire bibliography from the command line<br/>
  Inspired by the need for better academic paper management tools<br/>
  Built with Rust</i><br/></p>
  <p>
   <img alt="Static Badge" src="https://img.shields.io/badge/homebrew-black?style=for-the-badge&logo=homebrew&logoColor=white">

  </p>
</div>

# bib

A command-line research paper manager with AI-powered BibTeX extraction, semantic search, and intelligent paper analysis.

## Features

- **Smart Paper Import**: Add papers from arXiv URLs, direct PDF links, or local files
- **Automatic BibTeX Extraction**: Uses Gemini AI to extract and generate proper BibTeX entries
- **Interactive Search**: Fast fuzzy search with a terminal UI for browsing your library
- **Semantic Search**: Vector embedding-based search with LLM analysis for finding relevant papers
- **PDF Management**: Automatic PDF storage and organization with quick access
- **Clipboard Integration**: Copy BibTeX entries directly to clipboard
- **Database Storage**: SQLite backend with deduplication and efficient retrieval

## Installation

### Prerequisites

- A Google API key with Gemini access

### Homebrew (Recommended)

[![Homebrew](https://img.shields.io/badge/homebrew-tap-orange.svg)](https://github.com/antonio-leitao/homebrew-taps)

```bash
brew install antonio-leitao/taps/bib
```

### From Source

Requires Rust 1.70 or higher:

```bash
git clone https://github.com/antonio-leitao/bib.git
cd bib
cargo build --release
```

### Setup

Create a `.env` file in your home directory or project root:

```bash
GOOGLE_API_KEY=your_gemini_api_key_here
```

## Usage

### Adding Papers

Add a paper from various sources:

```bash
# From arXiv URL
bib add "https://arxiv.org/abs/2301.00001"

# From direct PDF URL
bib add "https://example.com/paper.pdf"

# From local PDF file
bib add /path/to/paper.pdf

# From clipboard (automatically detects URL or path)
bib add

# With notes
bib add "https://arxiv.org/abs/2301.00001" -n "Important for my thesis"
```

The tool will:

1. Download the PDF (if from URL)
2. Extract BibTeX using AI or DOI lookup
3. Generate vector embeddings for semantic search
4. Store the paper and PDF locally

### Interactive Search

Launch the interactive search interface:

```bash
bib search

# Limit displayed results
bib search -n 20
```

**Search Mode Commands:**

- Type to search papers
- `Enter` or `Tab`: Switch to browse mode
- `Esc`: Quit

**Browse Mode Commands:**

- `j`/`↓`: Move down
- `k`/`↑`: Move up
- `Enter`: Open PDF with default viewer
- `o`: Open PDF in browser
- `y`: Copy BibTeX to clipboard
- `d`: Delete paper (with confirmation)
- `Tab`: Return to search mode
- `q` or `Esc`: Quit

### Semantic Search with Analysis

Find papers using natural language queries and AI analysis:

```bash
# Search with semantic understanding
bib find "papers about transformer architectures in computer vision"

# Limit papers analyzed (default 20)
bib find "applications of topological data analysis" -n 10
```

This command:

1. Generates a query embedding
2. Finds semantically similar papers using vector search
3. Uploads top papers to Gemini for detailed analysis
4. Returns structured analysis with relevance scores and key findings

### Database Statistics

View storage statistics:

```bash
bib stats
```

## Data Storage

Bib stores data in your home directory:

```
~/.papers/
├── papers.db       # SQLite database with metadata and embeddings
└── pdfs/          # PDF files organized by paper ID
```

## Architecture

### Core Components

- **BibTeX Parser**: Robust parsing with content-based deduplication
- **Gemini Integration**:
  - BibTeX extraction from PDFs
  - Paper summarization for embeddings
  - Multi-paper analysis for complex queries
- **Vector Search**: Efficient k-nearest neighbor search with normalized embeddings
- **Storage Layer**: SQLite with support for papers and embeddings

### Key Technologies

- **Database**: SQLite with full-text search
- **AI/ML**: Google Gemini for NLP tasks
- **Embeddings**: 768-dimensional vectors for semantic similarity
- **UI**: Terminal-based interface with real-time fuzzy search
- **PDF Processing**: Automatic download, storage, and retrieval

## Troubleshooting

### Common Issues

**API Key Not Found**

```
Error: GOOGLE_API_KEY not found in environment variables
```

Solution: Ensure your `.env` file contains a valid Gemini API key.

**PDF Download Failures**

- Some publishers block automated downloads
- Try downloading manually and use the local file path
- Check if the paper is available on arXiv

**Database Errors**

- Ensure `~/.papers/` directory has write permissions
- Check disk space availability

## License

MIT License - see LICENSE file for details
