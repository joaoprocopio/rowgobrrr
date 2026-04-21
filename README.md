# beecrab

1 billion row challenge in rust.

## todo

- make it parallel, and process in chunks

## notes

- naive: b710b6070e94110aab47036afa382dfd1f4e40ca
- btree to hashmap: 36af5fa4d9e2850f8c6ae6939b1672593f112ba6
- memory mapping: 3a97d429c6af92ef2611a23f679b56a2e1abbc45

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
perf record -F99 --call-graph dwarf ./target/release/beecrab measurements.txt
```

### visualize perf data on a tui

```sh
perf report
```

### visualize perf data on a gui

> you need to install `rustfilt` with `cargo install rustfilt`
>
> this is needed to demangle rust symbol names

```sh
perf script | rustfilt > perf.txt
# paste the text file on: https://profiler.firefox.com/
```

## data insights

1. there is a total of `413` different stations
2. the longest station name is `Las Palmas de Gran Canaria` with `26` characters
3. the shortest station name is `Wau` and `Jos` with 3 characters
4. the distribution for station names looks something like:
   - mean: `7.8`
   - median: `7`
   - mode: `6`
5. temperatures are always in a inclusive range of `-99.9` to `99.9`
