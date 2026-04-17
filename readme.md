# beecrab

1 billion row challenge in rust.

## profilling

tools:

- perf linux
- callgrind
- valgrind
- cachegrind

build:

```sh
cargo build --release
```

record:

<!-- https://nnethercote.github.io/perf-book/profiling.html -->
<!-- https://rustc-dev-guide.rust-lang.org/profiling/with-perf.html -->

```sh
perf record -F99 --call-graph dwarf ./target/release/beecrab
```

visualize (TUI):

```sh
perf report
```

visualize (GUI):

```sh
perf script > perf.txt
# paste the text file on: https://profiler.firefox.com/
```
