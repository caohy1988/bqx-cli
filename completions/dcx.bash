_dcx() {
    local i cur prev opts cmd
    COMPREPLY=()
    if [[ "${BASH_VERSINFO[0]}" -ge 4 ]]; then
        cur="$2"
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
    fi
    prev="$3"
    cmd=""
    opts=""

    for i in "${COMP_WORDS[@]:0:COMP_CWORD}"
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="dcx"
                ;;
            dcx,analytics)
                cmd="dcx__analytics"
                ;;
            dcx,auth)
                cmd="dcx__auth"
                ;;
            dcx,ca)
                cmd="dcx__ca"
                ;;
            dcx,completions)
                cmd="dcx__completions"
                ;;
            dcx,datasets)
                cmd="dcx__datasets"
                ;;
            dcx,generate-skills)
                cmd="dcx__generate__skills"
                ;;
            dcx,help)
                cmd="dcx__help"
                ;;
            dcx,jobs)
                cmd="dcx__jobs"
                ;;
            dcx,models)
                cmd="dcx__models"
                ;;
            dcx,routines)
                cmd="dcx__routines"
                ;;
            dcx,tables)
                cmd="dcx__tables"
                ;;
            dcx__analytics,distribution)
                cmd="dcx__analytics__distribution"
                ;;
            dcx__analytics,doctor)
                cmd="dcx__analytics__doctor"
                ;;
            dcx__analytics,drift)
                cmd="dcx__analytics__drift"
                ;;
            dcx__analytics,evaluate)
                cmd="dcx__analytics__evaluate"
                ;;
            dcx__analytics,get-trace)
                cmd="dcx__analytics__get__trace"
                ;;
            dcx__analytics,help)
                cmd="dcx__analytics__help"
                ;;
            dcx__analytics,hitl-metrics)
                cmd="dcx__analytics__hitl__metrics"
                ;;
            dcx__analytics,insights)
                cmd="dcx__analytics__insights"
                ;;
            dcx__analytics,list-traces)
                cmd="dcx__analytics__list__traces"
                ;;
            dcx__analytics,views)
                cmd="dcx__analytics__views"
                ;;
            dcx__analytics__help,distribution)
                cmd="dcx__analytics__help__distribution"
                ;;
            dcx__analytics__help,doctor)
                cmd="dcx__analytics__help__doctor"
                ;;
            dcx__analytics__help,drift)
                cmd="dcx__analytics__help__drift"
                ;;
            dcx__analytics__help,evaluate)
                cmd="dcx__analytics__help__evaluate"
                ;;
            dcx__analytics__help,get-trace)
                cmd="dcx__analytics__help__get__trace"
                ;;
            dcx__analytics__help,help)
                cmd="dcx__analytics__help__help"
                ;;
            dcx__analytics__help,hitl-metrics)
                cmd="dcx__analytics__help__hitl__metrics"
                ;;
            dcx__analytics__help,insights)
                cmd="dcx__analytics__help__insights"
                ;;
            dcx__analytics__help,list-traces)
                cmd="dcx__analytics__help__list__traces"
                ;;
            dcx__analytics__help,views)
                cmd="dcx__analytics__help__views"
                ;;
            dcx__analytics__help__views,create-all)
                cmd="dcx__analytics__help__views__create__all"
                ;;
            dcx__analytics__views,create-all)
                cmd="dcx__analytics__views__create__all"
                ;;
            dcx__analytics__views,help)
                cmd="dcx__analytics__views__help"
                ;;
            dcx__analytics__views__help,create-all)
                cmd="dcx__analytics__views__help__create__all"
                ;;
            dcx__analytics__views__help,help)
                cmd="dcx__analytics__views__help__help"
                ;;
            dcx__auth,help)
                cmd="dcx__auth__help"
                ;;
            dcx__auth,login)
                cmd="dcx__auth__login"
                ;;
            dcx__auth,logout)
                cmd="dcx__auth__logout"
                ;;
            dcx__auth,status)
                cmd="dcx__auth__status"
                ;;
            dcx__auth__help,help)
                cmd="dcx__auth__help__help"
                ;;
            dcx__auth__help,login)
                cmd="dcx__auth__help__login"
                ;;
            dcx__auth__help,logout)
                cmd="dcx__auth__help__logout"
                ;;
            dcx__auth__help,status)
                cmd="dcx__auth__help__status"
                ;;
            dcx__ca,add-verified-query)
                cmd="dcx__ca__add__verified__query"
                ;;
            dcx__ca,ask)
                cmd="dcx__ca__ask"
                ;;
            dcx__ca,create-agent)
                cmd="dcx__ca__create__agent"
                ;;
            dcx__ca,help)
                cmd="dcx__ca__help"
                ;;
            dcx__ca,list-agents)
                cmd="dcx__ca__list__agents"
                ;;
            dcx__ca__help,add-verified-query)
                cmd="dcx__ca__help__add__verified__query"
                ;;
            dcx__ca__help,ask)
                cmd="dcx__ca__help__ask"
                ;;
            dcx__ca__help,create-agent)
                cmd="dcx__ca__help__create__agent"
                ;;
            dcx__ca__help,help)
                cmd="dcx__ca__help__help"
                ;;
            dcx__ca__help,list-agents)
                cmd="dcx__ca__help__list__agents"
                ;;
            dcx__datasets,get)
                cmd="dcx__datasets__get"
                ;;
            dcx__datasets,help)
                cmd="dcx__datasets__help"
                ;;
            dcx__datasets,list)
                cmd="dcx__datasets__list"
                ;;
            dcx__datasets__help,get)
                cmd="dcx__datasets__help__get"
                ;;
            dcx__datasets__help,help)
                cmd="dcx__datasets__help__help"
                ;;
            dcx__datasets__help,list)
                cmd="dcx__datasets__help__list"
                ;;
            dcx__help,analytics)
                cmd="dcx__help__analytics"
                ;;
            dcx__help,auth)
                cmd="dcx__help__auth"
                ;;
            dcx__help,ca)
                cmd="dcx__help__ca"
                ;;
            dcx__help,completions)
                cmd="dcx__help__completions"
                ;;
            dcx__help,datasets)
                cmd="dcx__help__datasets"
                ;;
            dcx__help,generate-skills)
                cmd="dcx__help__generate__skills"
                ;;
            dcx__help,help)
                cmd="dcx__help__help"
                ;;
            dcx__help,jobs)
                cmd="dcx__help__jobs"
                ;;
            dcx__help,models)
                cmd="dcx__help__models"
                ;;
            dcx__help,routines)
                cmd="dcx__help__routines"
                ;;
            dcx__help,tables)
                cmd="dcx__help__tables"
                ;;
            dcx__help__analytics,distribution)
                cmd="dcx__help__analytics__distribution"
                ;;
            dcx__help__analytics,doctor)
                cmd="dcx__help__analytics__doctor"
                ;;
            dcx__help__analytics,drift)
                cmd="dcx__help__analytics__drift"
                ;;
            dcx__help__analytics,evaluate)
                cmd="dcx__help__analytics__evaluate"
                ;;
            dcx__help__analytics,get-trace)
                cmd="dcx__help__analytics__get__trace"
                ;;
            dcx__help__analytics,hitl-metrics)
                cmd="dcx__help__analytics__hitl__metrics"
                ;;
            dcx__help__analytics,insights)
                cmd="dcx__help__analytics__insights"
                ;;
            dcx__help__analytics,list-traces)
                cmd="dcx__help__analytics__list__traces"
                ;;
            dcx__help__analytics,views)
                cmd="dcx__help__analytics__views"
                ;;
            dcx__help__analytics__views,create-all)
                cmd="dcx__help__analytics__views__create__all"
                ;;
            dcx__help__auth,login)
                cmd="dcx__help__auth__login"
                ;;
            dcx__help__auth,logout)
                cmd="dcx__help__auth__logout"
                ;;
            dcx__help__auth,status)
                cmd="dcx__help__auth__status"
                ;;
            dcx__help__ca,add-verified-query)
                cmd="dcx__help__ca__add__verified__query"
                ;;
            dcx__help__ca,ask)
                cmd="dcx__help__ca__ask"
                ;;
            dcx__help__ca,create-agent)
                cmd="dcx__help__ca__create__agent"
                ;;
            dcx__help__ca,list-agents)
                cmd="dcx__help__ca__list__agents"
                ;;
            dcx__help__datasets,get)
                cmd="dcx__help__datasets__get"
                ;;
            dcx__help__datasets,list)
                cmd="dcx__help__datasets__list"
                ;;
            dcx__help__jobs,query)
                cmd="dcx__help__jobs__query"
                ;;
            dcx__help__models,get)
                cmd="dcx__help__models__get"
                ;;
            dcx__help__models,list)
                cmd="dcx__help__models__list"
                ;;
            dcx__help__routines,get)
                cmd="dcx__help__routines__get"
                ;;
            dcx__help__routines,list)
                cmd="dcx__help__routines__list"
                ;;
            dcx__help__tables,get)
                cmd="dcx__help__tables__get"
                ;;
            dcx__help__tables,list)
                cmd="dcx__help__tables__list"
                ;;
            dcx__jobs,help)
                cmd="dcx__jobs__help"
                ;;
            dcx__jobs,query)
                cmd="dcx__jobs__query"
                ;;
            dcx__jobs__help,help)
                cmd="dcx__jobs__help__help"
                ;;
            dcx__jobs__help,query)
                cmd="dcx__jobs__help__query"
                ;;
            dcx__models,get)
                cmd="dcx__models__get"
                ;;
            dcx__models,help)
                cmd="dcx__models__help"
                ;;
            dcx__models,list)
                cmd="dcx__models__list"
                ;;
            dcx__models__help,get)
                cmd="dcx__models__help__get"
                ;;
            dcx__models__help,help)
                cmd="dcx__models__help__help"
                ;;
            dcx__models__help,list)
                cmd="dcx__models__help__list"
                ;;
            dcx__routines,get)
                cmd="dcx__routines__get"
                ;;
            dcx__routines,help)
                cmd="dcx__routines__help"
                ;;
            dcx__routines,list)
                cmd="dcx__routines__list"
                ;;
            dcx__routines__help,get)
                cmd="dcx__routines__help__get"
                ;;
            dcx__routines__help,help)
                cmd="dcx__routines__help__help"
                ;;
            dcx__routines__help,list)
                cmd="dcx__routines__help__list"
                ;;
            dcx__tables,get)
                cmd="dcx__tables__get"
                ;;
            dcx__tables,help)
                cmd="dcx__tables__help"
                ;;
            dcx__tables,list)
                cmd="dcx__tables__list"
                ;;
            dcx__tables__help,get)
                cmd="dcx__tables__help__get"
                ;;
            dcx__tables__help,help)
                cmd="dcx__tables__help__help"
                ;;
            dcx__tables__help,list)
                cmd="dcx__tables__help__list"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        dcx)
            opts="-h -V --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help --version jobs analytics auth ca generate-skills completions datasets models routines tables help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__distribution)
            opts="-h --last --agent-id --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --last)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --agent-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__doctor)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__drift)
            opts="-h --golden-dataset --last --agent-id --min-coverage --exit-code --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --golden-dataset)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --last)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --agent-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-coverage)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__evaluate)
            opts="-h --evaluator --threshold --last --agent-id --exit-code --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --evaluator)
                    COMPREPLY=($(compgen -W "latency error-rate" -- "${cur}"))
                    return 0
                    ;;
                --threshold)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --last)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --agent-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__get__trace)
            opts="-h --session-id --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --session-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__help)
            opts="doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__help__distribution)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__help__doctor)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__help__drift)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__help__evaluate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__help__get__trace)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__help__hitl__metrics)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__help__insights)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__help__list__traces)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__help__views)
            opts="create-all"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__help__views__create__all)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__hitl__metrics)
            opts="-h --last --agent-id --limit --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --last)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --agent-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__insights)
            opts="-h --last --agent-id --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --last)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --agent-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__list__traces)
            opts="-h --last --agent-id --limit --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --last)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --agent-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__views)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help create-all help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__views__create__all)
            opts="-h --prefix --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__views__help)
            opts="create-all help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__views__help__create__all)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__analytics__views__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__auth)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help login status logout help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__auth__help)
            opts="login status logout help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__auth__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__auth__help__login)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__auth__help__logout)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__auth__help__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__auth__login)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__auth__logout)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__auth__status)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__ca)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help ask create-agent list-agents add-verified-query help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__ca__add__verified__query)
            opts="-h --agent --question --query --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --agent)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --question)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --query)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__ca__ask)
            opts="-h --agent --tables --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help <QUESTION>"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --agent)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tables)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__ca__create__agent)
            opts="-h --name --tables --views --verified-queries --instructions --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --name)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tables)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --views)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --verified-queries)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --instructions)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__ca__help)
            opts="ask create-agent list-agents add-verified-query help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__ca__help__add__verified__query)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__ca__help__ask)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__ca__help__create__agent)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__ca__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__ca__help__list__agents)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__ca__list__agents)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__completions)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help bash zsh fish"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__datasets)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help get list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__datasets__get)
            opts="-h --access-policy-version --dataset-view --dry-run --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --access-policy-version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-view)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__datasets__help)
            opts="get list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__datasets__help__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__datasets__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__datasets__help__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__datasets__list)
            opts="-h --all --filter --max-results --page-token --dry-run --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-results)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --page-token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__generate__skills)
            opts="-h --output-dir --filter --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --output-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help)
            opts="jobs analytics auth ca generate-skills completions datasets models routines tables help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__analytics)
            opts="doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__analytics__distribution)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__analytics__doctor)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__analytics__drift)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__analytics__evaluate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__analytics__get__trace)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__analytics__hitl__metrics)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__analytics__insights)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__analytics__list__traces)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__analytics__views)
            opts="create-all"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__analytics__views__create__all)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__auth)
            opts="login status logout"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__auth__login)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__auth__logout)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__auth__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__ca)
            opts="ask create-agent list-agents add-verified-query"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__ca__add__verified__query)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__ca__ask)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__ca__create__agent)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__ca__list__agents)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__completions)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__datasets)
            opts="get list"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__datasets__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__datasets__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__generate__skills)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__jobs)
            opts="query"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__jobs__query)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__models)
            opts="get list"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__models__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__models__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__routines)
            opts="get list"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__routines__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__routines__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__tables)
            opts="get list"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__tables__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__help__tables__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__jobs)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help query help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__jobs__help)
            opts="query help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__jobs__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__jobs__help__query)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__jobs__query)
            opts="-h --query --use-legacy-sql --dry-run --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --query)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__models)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help get list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__models__get)
            opts="-h --model-id --dry-run --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --model-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__models__help)
            opts="get list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__models__help__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__models__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__models__help__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__models__list)
            opts="-h --max-results --page-token --dry-run --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --max-results)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --page-token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__routines)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help get list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__routines__get)
            opts="-h --routine-id --read-mask --dry-run --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --routine-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-mask)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__routines__help)
            opts="get list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__routines__help__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__routines__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__routines__help__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__routines__list)
            opts="-h --filter --max-results --page-token --read-mask --dry-run --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-results)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --page-token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-mask)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__tables)
            opts="-h --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help get list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__tables__get)
            opts="-h --table-id --selected-fields --view --dry-run --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --table-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --selected-fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --view)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__tables__help)
            opts="get list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__tables__help__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__tables__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__tables__help__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        dcx__tables__list)
            opts="-h --max-results --page-token --dry-run --project-id --dataset-id --location --table --format --token --credentials-file --sanitize --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --max-results)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --page-token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dataset-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --location)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --table)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "json table text" -- "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credentials-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sanitize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _dcx -o nosort -o bashdefault -o default dcx
else
    complete -F _dcx -o bashdefault -o default dcx
fi
