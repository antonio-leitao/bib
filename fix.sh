#!/bin/bash

# Script to fix bib zsh completions after Homebrew installation

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}Bib Completion Fix Script${NC}"
echo "========================="
echo

# 1. Check if bib is installed
if ! command -v bib >/dev/null 2>&1; then
    echo -e "${RED}Error: bib is not in PATH${NC}"
    echo "Please ensure bib is installed: brew install yourusername/taps/bib"
    exit 1
fi

echo -e "${GREEN}✓${NC} bib found at: $(which bib)"

# 2. Check if database exists
DB_PATH="$HOME/.bib/papers.db"
if [ ! -f "$DB_PATH" ]; then
    echo -e "${YELLOW}Warning: Database not found at $DB_PATH${NC}"
    echo "Completions won't work until you add at least one paper."
    echo "Try: bib add https://arxiv.org/abs/2301.01234"
else
    echo -e "${GREEN}✓${NC} Database found at: $DB_PATH"
    PAPER_COUNT=$(bib stats 2>/dev/null | grep "Total papers" | awk '{print $3}')
    echo "  Papers in database: $PAPER_COUNT"
fi

# 3. Find where Homebrew installed the completions
BREW_PREFIX=$(brew --prefix)
ZSH_COMPLETION_DIR="$BREW_PREFIX/share/zsh/site-functions"

if [ -f "$ZSH_COMPLETION_DIR/_bib" ]; then
    echo -e "${GREEN}✓${NC} Completion file found at: $ZSH_COMPLETION_DIR/_bib"
else
    echo -e "${RED}Error: Completion file not found at $ZSH_COMPLETION_DIR/_bib${NC}"
    echo "Try reinstalling: brew reinstall yourusername/taps/bib"
    exit 1
fi

# 4. Check if fpath includes the Homebrew completions
echo
echo "Checking your zsh configuration..."

if ! grep -q "FPATH.*$BREW_PREFIX/share/zsh/site-functions" ~/.zshrc 2>/dev/null; then
    echo -e "${YELLOW}Adding Homebrew completions to your ~/.zshrc...${NC}"
    cat >> ~/.zshrc << EOF

# Homebrew completions for zsh
if type brew &>/dev/null; then
  FPATH="$BREW_PREFIX/share/zsh/site-functions:\${FPATH}"
  autoload -Uz compinit
  compinit
fi
EOF
    echo -e "${GREEN}✓${NC} Added to ~/.zshrc"
else
    echo -e "${GREEN}✓${NC} Homebrew completions already in ~/.zshrc"
fi

# 5. Rebuild zsh completion cache
echo
echo -e "${YELLOW}Rebuilding zsh completion cache...${NC}"
rm -f ~/.zcompdump*
echo -e "${GREEN}✓${NC} Cache cleared"

# 6. Test the completion
echo
echo -e "${BLUE}Testing completion...${NC}"
if bib --complete "test" --complete-context "search-title" >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Completion command works!"
else
    echo -e "${YELLOW}Warning: Completion command failed${NC}"
    echo "This is normal if you haven't added any papers yet."
fi

# 7. Final instructions
echo
echo -e "${GREEN}Setup complete!${NC}"
echo
echo "To activate the changes:"
echo "  1. Restart your terminal, OR"
echo "  2. Run: source ~/.zshrc"
echo
echo "Then test with:"
echo "  bib search <TAB>"
echo
echo "If completions still don't work:"
echo "  1. Make sure you have papers in the database: bib stats"
echo "  2. Try manually: bib --complete \"\" --complete-context search-title"
echo "  3. Check for errors: bib --complete \"test\" --complete-context search-title 2>&1"
