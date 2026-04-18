# Plan

1. Replace the iterator pipeline in `src/main.rs` with a single manual byte parser over the mmap.
2. Remove `split_once()`, `from_utf8_unchecked()`, and `f64::parse()` from the hot path.
3. Parse temperatures as fixed-point integers in tenths and keep `min`, `max`, and `sum` as integers.
4. Reduce hashmap overhead by avoiding repeated `&str` hashing and string comparisons per row.
5. Split the file into newline-aligned chunks, process them in parallel, and merge thread-local aggregates.
6. Tighten release settings for benchmarks: disable full debug info, enable LTO, and build with `-C target-cpu=native`.
7. Re-profile after each step and only move to custom table/hash work if the simpler changes are not enough.
