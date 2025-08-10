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
                    # Check for author flag
                    local context="search-title"
                    if (( ${words[(I)-a]} )) || (( ${words[(I)--author]} )); then
                        context="search-author"
                    fi
                    
                    _arguments -C \
                        '(-a --author)'{-a,--author}'[Search by author instead of title]' \
                        '(-n --limit)'{-n,--limit}'[Maximum results]:limit:' \
                        '(-h --help)'{-h,--help}'[Show help]' \
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
