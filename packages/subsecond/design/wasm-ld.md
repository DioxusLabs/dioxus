OVERVIEW: LLVM Linker

USAGE: /Users/jonkelley/.rustup/toolchains/stable-aarch64-apple-darwin//lib/rustlib/aarch64-apple-darwin/bin/rust-lld [options] file...

OPTIONS:
  --allow-undefined-file=<value>
                          Allow symbols listed in <file> to be undefined in linked binary
  --allow-undefined       Allow undefined symbols in linked binary. This options is equivalent to --import-undefined and --unresolved-symbols=ignore-all
  --Bdynamic              Link against shared libraries
  --Bstatic               Do not link against shared libraries (default)
  --Bsymbolic             Bind defined symbols locally
  --build-id=[fast,sha1,uuid,0x<hexstring>]
                          Generate build ID note
  --build-id              Alias for --build-id=fast
  --call_shared           Alias for --Bdynamic
  --check-features        Check feature compatibility of linked objects (default)
  --color-diagnostics=[auto,always,never]
                          Use colors in diagnostics (default: auto)
  --color-diagnostics     Alias for --color-diagnostics=always
  --compress-relocations  Compress the relocation targets in the code section.
  --demangle              Demangle symbol names (default)
  --dn                    Alias for --Bstatic
  --dy                    Alias for --Bdynamic
  --emit-relocs           Generate relocations in output
  --end-lib               End a grouping of objects that should be treated as if they were together in an archive
  --entry <entry>         Name of entry point symbol
  --error-limit=<value>   Maximum number of errors to emit before stopping (0 = no limit)
  --error-unresolved-symbols
                          Report unresolved symbols as errors
  --experimental-pic      Enable Experimental PIC
  --export-all            Export all symbols (normally combined with --no-gc-sections)
  --export-dynamic        Put symbols in the dynamic symbol table
  --export-if-defined=<value>
                          Force a symbol to be exported, if it is defined in the input
  --export-memory=<value> Export the module's memory with the passed name
  --export-memory         Export the module's memory with the default name of "memory"
  --export-table          Export function table to the environment
  --export=<value>        Force a symbol to be exported
  --extra-features=<value>
                          Comma-separated list of features to add to the default set of features inferred from input objects.
  -E                      Alias for --export-dynamic
  --fatal-warnings        Treat warnings as errors
  --features=<value>      Comma-separated used features, inferred from input objects by default.
  --gc-sections           Enable garbage collection of unused sections (defualt)
  --global-base=<value>   Memory offset at which to place global data (Defaults to 1024)
  --growable-table        Remove maximum size from function table, allowing table to grow
  --help                  Print option help
  --import-memory=<module>,<name>
                          Import the module's memory from the passed module with the passed name.
  --import-memory         Import the module's memory from the default module of "env" with the name "memory".
  --import-table          Import function table from the environment
  --import-undefined      Turn undefined symbols into imports where possible
  --initial-heap=<value>  Initial size of the heap
  --initial-memory=<value>
                          Initial size of the linear memory
  --keep-section=<value>  Preserve a section even when --strip-all is given. This is useful for compiler drivers such as clang or emcc that, for example, depend on the features section for post-link processing. Can be specified multiple times to keep multiple sections
  --lto-CGO<cgopt-level>  Codegen optimization level for LTO
  --lto-debug-pass-manager
                          Debug new pass manager
  --lto-O<opt-level>      Optimization level for LTO
  --lto-partitions=<value>
                          Number of LTO codegen partitions
  -L <dir>                Add a directory to the library search path
  -l <libName>            Root name of library to use
  --Map=<value>           Print a link map to the specified file
  --max-memory=<value>    Maximum size of the linear memory
  --merge-data-segments   Enable merging data segments (default)
  --mllvm=<value>         Additional arguments to forward to LLVM's option processing
  -M                      Alias for --print-map
  -m <value>              Set target emulation
  --no-check-features     Ignore feature compatibility of linked objects
  --no-color-diagnostics  Alias for --color-diagnostics=never
  --no-demangle           Do not demangle symbol names
  --no-entry              Do not output any entry point
  --no-export-dynamic     Do not put symbols in the dynamic symbol table (default)
  --no-fatal-warnings     Do not treat warnings as errors (default)
  --no-gc-sections        Disable garbage collection of unused sections
  --no-growable-memory    Set maximum size of the linear memory to its initial size
  --no-merge-data-segments
                          Disable merging data segments
  --no-pie                Do not create a position independent executable (default)
  --no-print-gc-sections  Do not list removed unused sections (default)
  --no-shlib-sigcheck     Do not check signatures of functions defined in shared libraries.
  --no-whole-archive      Do not force load of all members in a static library (default)
  --non_shared            Alias for --Bstatic
  -O <value>              Optimize output file size
  -o <path>               Path to file to write output
  --pie                   Create a position independent executable
  --print-gc-sections     List removed unused sections
  --print-map             Print a link map to the standard output
  --relocatable           Create relocatable object file
  --reproduce=<value>     Dump linker invocation and input files for debugging
  --rsp-quoting=[posix,windows]
                          Quoting style for response files
  --save-temps            Save intermediate LTO compilation results
  --shared-memory         Use shared linear memory
  --shared                Build a shared object
  --soname=<value>        Set the module name in the generated name section
  --stack-first           Place stack at start of linear memory rather than after data
  --start-lib             Start a grouping of objects that should be treated as if they were together in an archive
  --static                Alias for --Bstatic
  --strip-all             Strip all symbols
  --strip-debug           Strip debugging information
  -S                      Alias for --strip-debug
  -s                      Alias for --strip-all
  --table-base=<value>    Table offset at which to place address taken functions (Defaults to 1)
  --thinlto-cache-dir=<value>
                          Path to ThinLTO cached object file directory
  --thinlto-cache-policy=<value>
                          Pruning policy for the ThinLTO cache
  --thinlto-jobs=<value>  Number of ThinLTO jobs. Default to --threads=
  --threads=<value>       Number of threads. '1' disables multi-threading. By default all available hardware threads are used
  --trace-symbol=<value>  Trace references to symbols
  --trace                 Print the names of the input files
  -t                      Alias for --trace
  --undefined=<value>     Force undefined symbol during linking
  --unresolved-symbols=<value>
                          Determine how to handle unresolved symbols
  --verbose               Verbose mode
  --version               Display the version number and exit
  -v                      Display the version number
  --warn-unresolved-symbols
                          Report unresolved symbols as warnings
  --whole-archive         Force load of all members in a static library
  --why-extract=<value>   Print to a file about why archive members are extracted
  --wrap=<symbol>=<symbol>
                          Use wrapper functions for symbol
  -y <value>              Alias for --trace-symbol
  -z <option>             Linker option extensions
