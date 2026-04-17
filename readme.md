# beecrab

1 billion row challenge in rust.

## development

### generating data

> to run this project you need to generate the base text file. read the [guide](1brc/README.md) here.

```sh
cd 1brc
./mvnw clean verify
./create_measurements.sh 1000000000
./calculate_average_baseline.sh > baseline.txt
mv measurements.txt ..
mv baseline.txt ..
```

### tools used for profilling

- perf linux
- callgrind
- valgrind
- cachegrind

### building the binary

```sh
cargo build --release
```

### sampling

<!-- https://nnethercote.github.io/perf-book/profiling.html -->
<!-- https://rustc-dev-guide.rust-lang.org/profiling/with-perf.html -->

```sh
perf record -F99 --call-graph dwarf ./target/release/beecrab
```

### visualize perf data on a tui

```sh
perf report
```

### visualize perf data on a gui

```sh
perf script > perf.txt
# paste the text file on: https://profiler.firefox.com/
```
