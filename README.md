# RpyKit

Set of tools to work with Ren'Py files and projects.

Very experimental. API can change at any moment. It's not even clear
at this point if CLI should be a set of multiple tools, or a big
mega-tool.

Currently provides a highly parallelized `.rpa` archive extractor.

Basic usage:

```
cargo build --release
./target/release/rpa extract --output-dir . /path/to/archive.rpa
```
