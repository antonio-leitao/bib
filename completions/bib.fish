# Fish completion for bib

# Disable file completion by default
complete -c bib -f

# Helper function to check if we're using a subcommand
function __fish_bib_using_command
    set -l cmd (commandline -opc)
    if test (count $cmd) -gt 1
        if test $argv[1] = $cmd[2]
            return 0
        end
    end
    return 1
end

# Helper function to get completions from bib
function __fish_bib_complete_papers
    set -l context $argv[1]
    set -l current (commandline -ct)
    
    # Dynamically find the bib command
    set -l bib_cmd (command -v bib)
    if not test -e "$bib_cmd"
        return 1
    end

    # Use the found command
    set -l completions ($bib_cmd --complete "$current" --complete-context "$context" 2>/dev/null)
    
    for comp in $completions
        set -l parts (string split -m 1 ":" -- $comp)
        if test (count $parts) -eq 2
            echo -e "$parts[1]\t$parts[2]"
        else if test (count $parts) -eq 1
            echo "$parts[1]"
        end
    end
end

# Main commands
complete -c bib -n "not __fish_seen_subcommand_from add search list stats" -a "add" -d "Add new reference from URL or PDF"
complete -c bib -n "not __fish_seen_subcommand_from add search list stats" -a "search" -d "Search papers using fuzzy matching"
complete -c bib -n "not __fish_seen_subcommand_from add search list stats" -a "list" -d "List all papers in the database"
complete -c bib -n "not __fish_seen_subcommand_from add search list stats" -a "stats" -d "Show database statistics"

# Add command - enable file completion for this command
complete -c bib -n "__fish_bib_using_command add" -r -F
complete -c bib -n "__fish_bib_using_command add" -s n -l notes -d "Optional notes"
complete -c bib -n "__fish_bib_using_command add" -s h -l help -d "Show help"

# Search command
complete -c bib -n "__fish_bib_using_command search" -s a -l author -d "Search by author"
complete -c bib -n "__fish_bib_using_command search" -s n -l limit -d "Maximum results"
complete -c bib -n "__fish_bib_using_command search" -s h -l help -d "Show help"

# Dynamic completions for search based on context
complete -c bib -n "__fish_bib_using_command search; and not __fish_seen_argument -s a -l author" \
    -a "(__fish_bib_complete_papers search-title)"
    
complete -c bib -n "__fish_bib_using_command search; and __fish_seen_argument -s a -l author" \
    -a "(__fish_bib_complete_papers search-author)"

# List command
complete -c bib -n "__fish_bib_using_command list" -s l -l limit -d "Maximum papers to display"
complete -c bib -n "__fish_bib_using_command list" -l limit -a "5 10 20 50 100"
complete -c bib -n "__fish_bib_using_command list" -s h -l help -d "Show help"

# Stats command
complete -c bib -n "__fish_bib_using_command stats" -s h -l help -d "Show help"
