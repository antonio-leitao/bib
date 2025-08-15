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

A command-line bibliography manager with intelligent paper extraction, semantic search, and deep content analysis capabilities.

<p align="center">
  <img src="assets/demo.gif" alt="bib demo" width="600" align="center">
</p>

## Overview

`bib` transforms how researchers manage their paper collections by combining traditional bibliography management with modern language understanding. Beyond simple keyword matching, it understands the conceptual relationships between papers, automatically extracts structured citations, and can analyze paper contents to answer specific research questions.

## Key Features

### Intelligent Paper Management

- **Automatic BibTeX Extraction**: Extracts complete citation information from PDFs using language models, with fallback to DOI resolution when available
- **Content-Based Deduplication**: Identifies duplicate papers even when metadata differs
- **Smart PDF Organization**: Automatically stores and organizes PDFs with consistent naming

### Advanced Search Capabilities

- **Interactive Fuzzy Search**: Real-time search interface with vim-style navigation
- **Semantic Search**: Find conceptually related papers using vector embeddings and similarity metrics
- **Deep Content Analysis**: Query paper contents for specific methodologies, results, or concepts

### Seamless Workflow Integration

- **Multiple Input Sources**: Import from arXiv, direct URLs, local files, or clipboard
- **BibTeX Export**: One-key copy to clipboard for LaTeX integration
- **Browser and System Integration**: Open PDFs in your preferred viewer

## Installation

### Via Homebrew

```bash
brew install antonio-leitao/taps/bib
```

### From Source

Requires Rust 1.70+

```bash
git clone https://github.com/antonio-leitao/bib.git
cd bib
cargo build --release
cargo install --path .
```

### Configuration

Set up your Google API key for AI features:

```bash
echo "GEMINI_KEY=your_api_key_here" >> ~/.env
```

## Usage

### Adding Papers

Import papers from various sources:

```bash
# From arXiv
bib add https://arxiv.org/abs/2301.00001

# From direct PDF URL
bib add https://proceedings.mlr.press/v139/paper.pdf

# From local file
bib add ~/Downloads/paper.pdf

# From clipboard (auto-detects URL or path)
bib add

# With annotations
bib add https://arxiv.org/abs/2301.00001 -n "Foundational work on attention mechanisms"
```

### Searching Your Library

#### Interactive Search

Launch the terminal interface for browsing:

```bash
bib          # Quick launch
bib search   # Explicit command
```

**Navigation:**

- Search mode: Type to filter papers in real-time
- `Tab`/`Enter`: Switch to browse mode
- `j`/`k` or arrows: Navigate results
- `Enter`: Open PDF
- `y`: Copy BibTeX citation
- `d`: Delete paper
- `Esc`: Exit

#### Semantic Search

Find papers by concept rather than keywords:

```bash
bib find "transformer architectures in computer vision"
bib find "statistical methods for causal inference" -n 15
bib find "protein folding predictions" -t 0.8  # Higher threshold for precision
```

#### Deep Content Analysis

Analyze paper contents to answer specific questions:

```bash
bib scan "experimental results on ImageNet"
bib scan "papers comparing supervised vs self-supervised learning"
bib scan "applications of graph neural networks" -n 25
```

### Managing Your Library

View statistics:

```bash
bib stats
```

## Architecture

### Storage Layer

Papers and metadata are stored in SQLite with companion PDF storage:

```
~/.bib/
├── papers.db      # Metadata, embeddings, and indices
└── pdfs/          # Organized PDF collection
```

### Technical Components

The system leverages several sophisticated techniques:

- **Vector Embeddings**: 768-dimensional representations capture semantic meaning
- **Content Processing**: Multi-stage pipeline for text extraction and analysis
- **Similarity Search**: Efficient k-NN search with configurable thresholds
- **Structured Extraction**: Schema-guided extraction ensures consistent metadata

### Language Model Integration

The tool integrates with Google's Gemini models for:

- Citation extraction from unstructured PDFs
- Document summarization for embedding generation
- Multi-document analysis and synthesis
- Query understanding and expansion

## Examples

### Research Workflow

```bash
# Monday: Found interesting paper on Twitter
bib add https://arxiv.org/abs/2401.00001

# Tuesday: Import papers from bibliography
bib add paper1.pdf
bib add paper2.pdf

# Wednesday: Find related work
bib find "similar approaches to variational inference"

# Thursday: Investigate specific methodology
bib scan "papers using contrastive learning for embeddings"

# Friday: Export citations for paper
bib search  # Interactive search, press 'y' to copy citations
```

### Building a Reading List

```bash
# Find foundational papers
bib find "seminal work on neural networks" -n 20

# Find recent developments
bib find "2024 advances in large language models"

# Find papers with specific datasets
bib scan "experiments on COCO dataset"
```

## Performance Considerations

- **Embedding Generation**: First-time paper import generates embeddings (2-5 seconds)
- **Semantic Search**: Near-instantaneous once embeddings are cached
- **Content Analysis**: Processes ~5-10 papers per minute depending on length
- **Storage**: Approximately 50-100KB per paper including embeddings

## Troubleshooting

### API Key Issues

If you encounter API errors:

1. Verify your API key is set: `echo $GEMINI_KEY`
2. Check API quotas altoguth `bib` makes sure to be within free-tier quotas

### Import Failures

Some publishers restrict automated downloads. Workarounds:

- Download manually and import the local file
- Check if the paper is available on arXiv
- Use institutional access through your browser

### Search Performance

For large libraries (1000+ papers):

- Use higher thresholds (`-t 0.8`) for faster filtering
- Limit initial results (`-n 10`) then refine
- Consider the trade-off between recall and precision

## Contributing

Contributions are welcome. Please ensure:

- Code follows Rust conventions (`cargo fmt`, `cargo clippy`)
- Tests pass (`cargo test`)
- Documentation is updated for new features

## License

MIT License. See LICENSE file for details.

## Acknowledgments

Built with:

- [biblatex-rs](https://github.com/typst/biblatex) for BibTeX parsing
- [Google Gemini](https://ai.google.dev/) for language understanding
- [sqlite](https://www.sqlite.org/) for reliable local storage
- The Rust ecosystem for performance and reliability

## Author

Antonio Leitao

---

For bug reports and feature requests, please use the [issue tracker](https://github.com/antonio-leitao/bib/issues).
