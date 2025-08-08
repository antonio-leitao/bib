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
