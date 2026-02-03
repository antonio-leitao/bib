<p align="center">
  <img src='assets/logo.svg' width='250px' align="center"></img>
</p>

<div align="center">
<h3 max-width='200px' align="center"><code>bib</code></h3>
  <p><i>Search papers by how researchers cite them</i></p>
  <p>
   <img alt="Static Badge" src="https://img.shields.io/badge/homebrew-black?style=for-the-badge&logo=homebrew&logoColor=white">
  </p>
</div>

A citation knowledge base that lets you find papers based on how the research community describes them—not just their titles or abstracts.

## The Idea

When researchers cite a paper, they describe it in context: what it does, why it matters, how it relates to their work.
These citation contexts capture how the community actually thinks about and uses a paper.

**`bib` builds a searchable database of these citation contexts.**

When you add a paper, the system extracts every paragraph that cites other work.
Each paragraph captures how the source paper describes the cited papers.
When you query, you're searching through these descriptions—finding papers based on how other researchers characterize them.

### Example

Say Paper A contains this paragraph:

> Recent advances in topological data analysis for protein structure [smith2020] have enabled new approaches to understanding folding dynamics.

When you query: `"topological data analysis for proteins"`

The system finds: **smith2020** — because Paper A described it that way.

### Why This Works

This is "crowdsourced" citation discovery.
A single paper's title might not mention "proteins" at all, but if dozens of papers cite it in the context of protein analysis, that pattern emerges.
The more papers you add, the richer and more accurate the citation contexts become.

## How It Works

### Building the Database

```
PDF → Extract paragraphs with citations → Embed citation contexts → Store
```

1. **Add PDFs** from arXiv, URLs, or local files
2. **Parse** each paper to find paragraphs containing citations
3. **Embed** each citation context capturing how the source describes the cited work
4. **Index** everything for fast semantic search

### Querying

```
Query → Match against citation contexts → LLM reranks → Return cited papers
```

1. Your query is embedded and matched against stored citation contexts
2. Results are the _cited papers_ that have been described in ways matching your query
3. An LLM reranks to identify the most relevant matches

## Commands

### Core Workflow

```bash
# Add papers to build your citation knowledge base
bib add https://arxiv.org/abs/2301.00001    # From arXiv
bib add https://example.com/paper.pdf        # From URL
bib add ~/Downloads/paper.pdf                # Local PDF
bib add                                      # From clipboard

# Batch process a directory of PDFs
bib sync ~/Papers/

# Search by citation context (the main feature)
bib query "topological methods for protein folding"
bib query "attention mechanisms in vision" -n 20

# Interactive fuzzy search UI
bib search
```

### Navigation (Interactive Search)

- Type to filter papers in real-time
- `Tab`/`Enter`: Switch to browse mode
- `j`/`k` or arrows: Navigate results
- `Enter`: Open PDF/Url
- `p`: Copy PDF to location
- `d`: Delete paper
- `Esc`: Exit

### Utilities

```bash
bib status    # Database statistics
bib config    # Setup storage directories
```

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

Set up your Gemini API key:

```bash
export GEMINI_KEY=your_api_key_here
```

Or add to your shell profile for persistence.

## Technical Notes

- **PDF Parsing**: Uses [Grobid](https://github.com/kermitt2/grobid) for structured extraction of citations and paragraphs
- **Embeddings & Reranking**: Google Gemini for semantic embeddings and LLM-based reranking
- **Storage**: SQLite database with companion PDF storage

## License

MIT License. See LICENSE file for details.

## Author

Antonio Leitao

---

For bug reports and feature requests, please use the [issue tracker](https://github.com/antonio-leitao/bib/issues).
