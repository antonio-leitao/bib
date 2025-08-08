#!/bin/bash

# Release script for bib
# This script helps create a new release and triggers the Homebrew tap update

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep "^version" Cargo.toml | head -1 | cut -d'"' -f2)

echo -e "${BLUE}Bib Release Script${NC}"
echo "===================="
echo
echo -e "Current version: ${YELLOW}$CURRENT_VERSION${NC}"
echo

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo -e "${RED}Error: You have uncommitted changes!${NC}"
    echo "Please commit or stash your changes before creating a release."
    exit 1
fi

# Prompt for new version
read -p "Enter new version (e.g., 0.2.0): " NEW_VERSION

if [ -z "$NEW_VERSION" ]; then
    echo -e "${RED}Error: Version cannot be empty${NC}"
    exit 1
fi

# Validate version format
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}Error: Version must be in format X.Y.Z${NC}"
    exit 1
fi

echo
echo -e "${GREEN}Preparing release v$NEW_VERSION...${NC}"
echo

# Update version in Cargo.toml
echo "1. Updating version in Cargo.toml..."
sed -i.bak "s/version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
rm Cargo.toml.bak

# Update Cargo.lock
echo "2. Updating Cargo.lock..."
cargo update

# Run tests
echo "3. Running tests..."
cargo test --all --quiet || echo "No tests found, skipping..."

# Build to verify
echo "4. Building release to verify..."
cargo build --release

# Generate completion files
echo "5. Generating completion files..."
mkdir -p completions

# Generate zsh completion
cat > completions/_bib << 'EOF'
#compdef bib

_bib() {
    local curcontext="$curcontext" state line
    typeset -A opt_args

    _arguments -C \
        '1: :->command' \
        '*:: :->args'

    case $state in
        command)
            local commands=(
                'add:Add new reference from URL or PDF'
                'search:Search papers using fuzzy matching'
                'list:List all papers in the database'
                'stats:Show database statistics'
            )
            _describe 'command' commands
            ;;
        args)
            case $line[1] in
                add)
                    _arguments \
                        '1:url:_files' \
                        '(-n --notes)'{-n,--notes}'[Optional notes]:notes:' \
                        '(-h --help)'{-h,--help}'[Show help]'
                    ;;
                search)
                    _arguments -C \
                        '(-a --author)'{-a,--author}'[Search by author instead of title]' \
                        '(-n --limit)'{-n,--limit}'[Maximum results]:limit:' \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '*:query:->search_query'
                    
                    case $state in
                        search_query)
                            local context="search-title"
                            if [[ ${opt_args[-a]} == 1 ]] || [[ ${opt_args[--author]} == 1 ]]; then
                                context="search-author"
                            fi
                            
                            local current_word="${words[CURRENT]}"
                            local completions
                            completions=(${(f)"$(bib --complete "$current_word" --complete-context "$context" 2>/dev/null)"})
                            
                            if [[ ${#completions[@]} -gt 0 ]]; then
                                _describe 'papers' completions
                            else
                                _message 'search query'
                            fi
                            ;;
                    esac
                    ;;
                list)
                    _arguments \
                        '(-l --limit)'{-l,--limit}'[Maximum papers to display]:limit:' \
                        '(-h --help)'{-h,--help}'[Show help]'
                    ;;
                stats)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]'
                    ;;
            esac
            ;;
    esac
}

_bib "$@"
EOF

# Generate bash completion
cat > completions/bib.bash << 'EOF'
# Bash completion for bib

_bib() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    # Main commands
    local commands="add search list stats"

    case "${prev}" in
        bib)
            COMPREPLY=( $(compgen -W "${commands}" -- ${cur}) )
            return 0
            ;;
        add)
            # File completion for add command
            COMPREPLY=( $(compgen -f -- ${cur}) )
            return 0
            ;;
        search)
            # Try to get completions from bib itself
            if command -v bib >/dev/null 2>&1; then
                local completions=$(bib --complete "${cur}" --complete-context "search-title" 2>/dev/null | cut -d: -f1)
                COMPREPLY=( $(compgen -W "${completions}" -- ${cur}) )
            fi
            return 0
            ;;
        -n|--notes)
            # No completion for notes
            return 0
            ;;
        -l|--limit)
            # Number completion
            COMPREPLY=( $(compgen -W "5 10 20 50 100" -- ${cur}) )
            return 0
            ;;
    esac

    # Check for flags
    case "${cur}" in
        -*)
            case "${COMP_WORDS[1]}" in
                add)
                    COMPREPLY=( $(compgen -W "-n --notes -h --help" -- ${cur}) )
                    ;;
                search)
                    COMPREPLY=( $(compgen -W "-a --author -n --limit -h --help" -- ${cur}) )
                    ;;
                list)
                    COMPREPLY=( $(compgen -W "-l --limit -h --help" -- ${cur}) )
                    ;;
                stats)
                    COMPREPLY=( $(compgen -W "-h --help" -- ${cur}) )
                    ;;
            esac
            return 0
            ;;
    esac
}

complete -F _bib bib
EOF

# Generate fish completion
cat > completions/bib.fish << 'EOF'
# Fish completion for bib

# Disable file completion by default
complete -c bib -f

# Main commands
complete -c bib -n "__fish_use_subcommand" -a "add" -d "Add new reference from URL or PDF"
complete -c bib -n "__fish_use_subcommand" -a "search" -d "Search papers using fuzzy matching"
complete -c bib -n "__fish_use_subcommand" -a "list" -d "List all papers in the database"
complete -c bib -n "__fish_use_subcommand" -a "stats" -d "Show database statistics"

# Add command
complete -c bib -n "__fish_seen_subcommand_from add" -F -d "URL or file path"
complete -c bib -n "__fish_seen_subcommand_from add" -s n -l notes -d "Optional notes"
complete -c bib -n "__fish_seen_subcommand_from add" -s h -l help -d "Show help"

# Search command
complete -c bib -n "__fish_seen_subcommand_from search" -s a -l author -d "Search by author"
complete -c bib -n "__fish_seen_subcommand_from search" -s n -l limit -d "Maximum results"
complete -c bib -n "__fish_seen_subcommand_from search" -s h -l help -d "Show help"

# Dynamic completions for search
complete -c bib -n "__fish_seen_subcommand_from search; and not __fish_seen_argument -s a -l author" \
    -a "(bib --complete (commandline -ct) --complete-context search-title 2>/dev/null | string replace ':' \t)"

complete -c bib -n "__fish_seen_subcommand_from search; and __fish_seen_argument -s a -l author" \
    -a "(bib --complete (commandline -ct) --complete-context search-author 2>/dev/null | string replace ':' \t)"

# List command
complete -c bib -n "__fish_seen_subcommand_from list" -s l -l limit -d "Maximum papers to display"
complete -c bib -n "__fish_seen_subcommand_from list" -s h -l help -d "Show help"

# Stats command
complete -c bib -n "__fish_seen_subcommand_from stats" -s h -l help -d "Show help"
EOF

echo -e "${GREEN}✓${NC} Completion files generated"

# Update CHANGELOG.md
echo "6. Updating CHANGELOG.md..."
if [ ! -f CHANGELOG.md ]; then
    cat > CHANGELOG.md << EOF
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [$NEW_VERSION] - $(date +%Y-%m-%d)

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

EOF
else
    # Add new version section after ## [Unreleased]
    sed -i.bak "/## \[Unreleased\]/a\\
\\
## [$NEW_VERSION] - $(date +%Y-%m-%d)\\
\\
### Added\\
- (Add your changes here)\\
\\
### Changed\\
- (Add your changes here)\\
\\
### Fixed\\
- (Add your changes here)" CHANGELOG.md
    rm CHANGELOG.md.bak
    
    echo
    echo -e "${YELLOW}Please edit CHANGELOG.md to add your changes for this release${NC}"
    echo "Opening CHANGELOG.md in your editor..."
    ${EDITOR:-vi} CHANGELOG.md
fi

# Create README if it doesn't exist
if [ ! -f README.md ]; then
    echo "7. Creating README.md..."
    cat > README.md << 'EOF'
# Bib - Academic Paper Manager

A command-line tool for managing academic papers with automatic BibTeX extraction, fuzzy search, and smart completions.

## Features

- 📚 **Smart Paper Import**: Add papers from arXiv URLs, PDF URLs, or local files
- 🤖 **Automatic BibTeX Extraction**: Uses AI to extract citation information from PDFs
- 🔍 **Fuzzy Search**: Fast, intelligent search through your paper collection
- 🎯 **Smart Completions**: Native shell completions with fuzzy matching
- 📦 **Local Storage**: SQLite database with content-based deduplication
- ⚡ **Fast & Lightweight**: Written in Rust for maximum performance

## Installation

### Homebrew (recommended)

```bash
brew install yourusername/taps/bib
```

### From Source

```bash
cargo install --path .
```

## Usage

### Add a Paper

```bash
# From clipboard (automatically detects URLs or paths)
bib add

# From specific URL
bib add "https://arxiv.org/abs/2301.01234"

# From local file
bib add ~/papers/paper.pdf

# With notes
bib add "https://arxiv.org/abs/2301.01234" -n "Important for my research"
```

### Search Papers

```bash
# Fuzzy search by title
bib search "neural networks"

# Search by author
bib search -a "Hinton"

# Limit results
bib search "deep learning" -n 5
```

### Other Commands

```bash
# List all papers
bib list

# Show statistics
bib stats
```

## Shell Completions

Completions are automatically installed with Homebrew. For manual installation:

```bash
# Zsh
mkdir -p ~/.config/bib/completions
cp completions/_bib ~/.config/bib/completions/
echo 'fpath=($HOME/.config/bib/completions $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc

# Bash
cp completions/bib.bash ~/.local/share/bash-completion/completions/
echo 'source ~/.local/share/bash-completion/completions/bib.bash' >> ~/.bashrc

# Fish
cp completions/bib.fish ~/.config/fish/completions/
```

## License

MIT
EOF
    echo -e "${GREEN}✓${NC} README.md created"
fi

# Commit changes
echo
echo "8. Committing version changes..."
git add Cargo.toml Cargo.lock CHANGELOG.md README.md completions/
git commit -m "Release version $NEW_VERSION

- Update version to $NEW_VERSION
- Update CHANGELOG.md
- Generate shell completions"

# Create and push tag
echo "9. Creating git tag v$NEW_VERSION..."
git tag -a "v$NEW_VERSION" -m "Release version $NEW_VERSION

See CHANGELOG.md for details."

# Push changes and tag
echo "10. Pushing to remote..."
git push origin main
git push origin "v$NEW_VERSION"

echo
echo -e "${GREEN}✅ Release v$NEW_VERSION created successfully!${NC}"
echo
echo "The GitHub Actions workflow will now:"
echo "  1. Build binaries for macOS (Intel & ARM) and Linux"
echo "  2. Create a GitHub release with the binaries"
echo "  3. Update the Homebrew tap automatically"
echo "  4. Install shell completions via Homebrew"
echo
echo "You can monitor the progress at:"
echo -e "${BLUE}https://github.com/$(git remote get-url origin | sed 's/.*github.com[:\/]\(.*\)\.git/\1/')/actions${NC}"
echo
echo "Once complete, users can install with:"
echo -e "  ${GREEN}brew install yourusername/taps/bib${NC}"
echo
echo "Or tap first:"
echo -e "  ${GREEN}brew tap yourusername/taps${NC}"
echo -e "  ${GREEN}brew install bib${NC}"
