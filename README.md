# Dataprep

## Dependencies
- cargo
- rustup

## Build the binary!
```bash
cargo build --bin dataprep
```

## Run the binary!
Pack a file:

```bash
dataprep pack --input-dir <INPUT_DIR> --output-dir <OUTPUT_DIR> --manifest-file <MANIFEST_FILE>
```

Unpack a file:

```bash
dataprep unpack --input-dir <INPUT_DIR> --manifest-file <MANIFEST_FILE> --output-dir <OUTPUT_DIR>
```

## Format

Make sure to run `cargo fmt` before committing. Run it in the package you want to format. (like, go into the `dataprep` directory and run `cargo fmt`)

```bash
cargo fmt # format code
cargo clippy # lint code
```

## Test

```bash
cargo test
```
