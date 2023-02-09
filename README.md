# Dataprep
[![codecov](https://codecov.io/gh/banyancomputer/dataprep/branch/master/graph/badge.svg?token=LQL6MA4KSI)](https://codecov.io/gh/banyancomputer/dataprep)
## Dependencies
- cargo
- rustup
- rust +nightly
- cargo-criterion
- docker

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

for unit tests
```bash
cargo test
```
for integration tests
```bash
cargo test --test integration
```

## Benchmark
Benchmarks are identified by
`Throughput/<pack or unpack>/<test set name>`

### Throughput
```bash
# Bench the packer with a 1Kb filestructure
cargo +nightly criterion --bench pipeline
```
You should see results in the `target/criterion/report/index.html` file.
### Profiling
dependency: `linux-perf`

If you are on a linux machine, you can use `perf` to profile the code.
We already do this for you! Just run the following command:
```bash
cargo +nightly bench --bench pipeline -- --profile-time 30 
```
This will run the pack and unpack benchmarks for 30 seconds each, and then generate a flamegraph for you.
The flamegraph will be in the `target/criterion/<name-of-benchmark>/profile/flamegraph.svg` file.

Otherwise, if you are on a mac but wanna get a sense of the performance, you can run the following command:
```bash
# Build the pipeline docker image
docker build -t pipeline -f dataprep-pipelines/Dockerfile .
```
```bash
# Create a volume to mount the code into the docker container
docker volume create dataprep
```
```bash
# Run the pipeline docker image and open a shell
docker run -it --rm -v dataprep-bench:/dataprep -w /dataprep dataprep-pipeline
```
This will open a shell in the docker container. Now, you can run the benchmarks:
```bash
# Run the throughput benchmark (if you want)
./run.sh
# Run the profiling benchmark
./run_profiling.sh
```
These commands should act the same as the ones above, but they will run inside the docker container.

TODO (amiller68) : I thought there'd be an easy way to get the flamegraph out of the docker container, but I couldn't figure it out. If you know how, please let me know!
In the meantime
```bash
# While the container is still running, open a new terminal
# Get the container id
docker container ls
# Copy the flamegraph out of the container
docker cp <container-id>:/dataprep/target/criterion/<name-of-benchmark>/profile/flamegraph.svg .
```
For the default benchmarks the name of the pack benchmark is `Throughput/pack/w4_d4_s1024_balanced`
and the name of the unpack benchmark is `Throughput/unpack/w4_d4_s1024_balanced`
So:
```bash
docker cp <container-id>:/dataprep/target/criterion/Throughput/pack/w4_d4_s1024_balanced/profile/flamegraph.svg .
```
or
```bash
docker cp <container-id>:/dataprep/target/criterion/Throughput/unpack/w4_d4_s1024_balanced/profile/flamegraph.svg .
```

Here's an exmaple of what the flamegraph looks like. I made this using the docker container, so it might not be representative of the performance on your machine.
![flamegraph](.github/flamegraph.svg)







