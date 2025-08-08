#!/bin/bash
# Bash completion for bib

_bib() {
    local cur prev words cword
    _init_completion || return

    local commands="add search list stats"

    # First argument - command selection
    if [[ $cword -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "${commands}" -- "${cur}") )
        return 0
    fi

    # Get the command (first argument after 'bib')
    local cmd="${words[1]}"

    case "${cmd}" in
        add)
            case "${prev}" in
                -n|--notes)
                    # No completion for notes
                    return 0
                    ;;
                add)
                    # File completion for first argument after add
                    _filedir
                    return 0
                    ;;
                *)
                    # Offer flags if we're typing a flag
                    if [[ ${cur} == -* ]]; then
                        COMPREPLY=( $(compgen -W "-n --notes -h --help" -- "${cur}") )
                    else
                        _filedir
                    fi
                    return 0
                    ;;
            esac
            ;;
            
        search)
            case "${prev}" in
                -n|--limit)
                    COMPREPLY=( $(compgen -W "5 10 20 50 100" -- "${cur}") )
                    return 0
                    ;;
                search)
                    # Get completions from bib for first argument after search
                    if command -v bib >/dev/null 2>&1; then
                        local completions=$(bib --complete "${cur}" --complete-context "search-title" 2>/dev/null)
                        if [[ -n "$completions" ]]; then
                            # Extract just the completion values (before the colon)
                            local IFS=$'\n'
                            local items=()
                            for comp in $completions; do
                                # Get everything before the first colon
                                local value="${comp%%:*}"
                                items+=("$value")
                            done
                            COMPREPLY=( $(compgen -W "${items[*]}" -- "${cur}") )
                        fi
                    fi
                    return 0
                    ;;
                *)
                    # Check if -a or --author was used to determine context
                    local context="search-title"
                    local i
                    for (( i=2; i < cword; i++ )); do
                        if [[ "${words[i]}" == "-a" ]] || [[ "${words[i]}" == "--author" ]]; then
                            context="search-author"
                            break
                        fi
                    done
                    
                    # Offer flags if typing a flag
                    if [[ ${cur} == -* ]]; then
                        COMPREPLY=( $(compgen -W "-a --author -n --limit -h --help" -- "${cur}") )
                    else
                        # Get completions for query
                        if command -v bib >/dev/null 2>&1; then
                            local completions=$(bib --complete "${cur}" --complete-context "${context}" 2>/dev/null)
                            if [[ -n "$completions" ]]; then
                                local IFS=$'\n'
                                local items=()
                                for comp in $completions; do
                                    local value="${comp%%:*}"
                                    items+=("$value")
                                done
                                COMPREPLY=( $(compgen -W "${items[*]}" -- "${cur}") )
                            fi
                        fi
                    fi
                    return 0
                    ;;
            esac
            ;;
            
        list)
            case "${prev}" in
                -l|--limit)
                    COMPREPLY=( $(compgen -W "5 10 20 50 100" -- "${cur}") )
                    return 0
                    ;;
                *)
                    if [[ ${cur} == -* ]]; then
                        COMPREPLY=( $(compgen -W "-l --limit -h --help" -- "${cur}") )
                    fi
                    return 0
                    ;;
            esac
            ;;
            
        stats)
            if [[ ${cur} == -* ]]; then
                COMPREPLY=( $(compgen -W "-h --help" -- "${cur}") )
            fi
            return 0
            ;;
    esac
}

complete -F _bib bib
