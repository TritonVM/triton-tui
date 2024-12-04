complete -c triton-tui -s i -l input -d 'File containing public input' -r
complete -c triton-tui -s n -l non-determinism -d 'JSON file containing all non-determinism' -r
complete -c triton-tui -l initial-state -d 'JSON file containing an entire VM state, including program and inputs. Conflicts with command line options program, input, and non-determinism' -r
complete -c triton-tui -l interrupt-cycle -d 'The maximum number of cycles to run after any interaction, preventing a frozen TUI in infinite loops' -r
complete -c triton-tui -s h -l help -d 'Print help'
complete -c triton-tui -s V -l version -d 'Print version'
