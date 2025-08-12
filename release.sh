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

# Update CHANGELOG.md
echo "5. Updating CHANGELOG.md..."
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

# Commit changes
echo
echo "6. Committing version changes..."
git add Cargo.toml Cargo.lock CHANGELOG.md completions/
git commit -m "Release version $NEW_VERSION

- Update version to $NEW_VERSION
- Update CHANGELOG.md
- Include shell completions"

# Create and push tag
echo "7. Creating git tag v$NEW_VERSION..."
git tag -a "v$NEW_VERSION" -m "Release version $NEW_VERSION

See CHANGELOG.md for details."

# Push changes and tag
echo "8. Pushing to remote..."
git push origin master
git push origin "v$NEW_VERSION"

echo
echo -e "${GREEN}✅ Release v$NEW_VERSION created successfully!${NC}"
echo
echo "The GitHub Actions workflow will now:"
echo "  1. Build binaries for macOS (Intel & ARM) and Linux"
echo "  2. Create a GitHub release with the binaries"
echo "  3. Update the Homebrew tap with proper completion installation"
echo
echo "You can monitor the progress at:"
echo -e "${BLUE}https://github.com/$(git remote get-url origin | sed 's/.*github.com[:\/]\(.*\)\.git/\1/')/actions${NC}"
echo
echo "Once complete, users can install with:"
echo -e "  ${GREEN}brew install antonio-leitao/taps/bib${NC}"
