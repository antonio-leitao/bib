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
