use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use dataprep_pipelines::{
    fsutil::ensure_path_exists_and_is_empty_dir,
    pipeline::{
        pack_pipeline::pack_pipeline,
        unpack_pipeline::unpack_pipeline,
    },
};
use std::path::PathBuf;
use tokio::runtime::Runtime;
use fs_extra::dir;

const INPUT_DIR: &str = "test/input";
const PACKED_DIR: &str = "test/packed";
const UNPACKED_DIR: &str = "test/unpacked";
const MANIFEST_FILE: &str = "test/manifest.json";

// TODO: Integrate with thea-exe's work on dataset generation
/// Make sure the input directory contains a prepared test set
// fn populate_input_dir() {
//     // Make sure the input directory exists and is empty.
//     ensure_path_exists_and_is_empty_dir(&PathBuf::from(INPUT_DIR)).unwrap();
//     // Create a file in the input directory
//     std::fs::write("test/input/test.txt", b"test").unwrap();
// }

/// Make sure packed directory and manifest file are empty for packing
fn prep_pack(packed_dir: &PathBuf, manifest_file: &PathBuf) {
    // if the path exists, remove it
    if packed_dir.exists() {
        std::fs::remove_dir_all(packed_dir).unwrap();
    }
    // if the manifest file exists, remove it
    if manifest_file.exists() {
        std::fs::remove_file(manifest_file).unwrap();
    }
    // Make sure the output directory exists and is empty
    ensure_path_exists_and_is_empty_dir(&packed_dir).unwrap();
}

/// Make sure the unpacked directory is empty for unpacking and that the manifest file exists
fn prep_unpack(unpacked_dir: &PathBuf, manifest_file: &PathBuf) {
    // if the path exists, remove it
    if unpacked_dir.exists() {
        std::fs::remove_dir_all(unpacked_dir).unwrap();
    }
    // Make sure the output directory exists and is empty
    ensure_path_exists_and_is_empty_dir(&unpacked_dir).unwrap();
    // Make sure the manifest file exists
    assert!(manifest_file.exists());
}

/// Benchmark packing - relies on INPUT_DIR being populated
fn pack_benchmark(c: &mut Criterion) {
    // TODO: Populate the input directory
    // populate_input_dir();
    // Create a runtime for the async function

    // Figure out how many bytes are in the input directory
    let input_dir_size = dir::get_size(INPUT_DIR).unwrap();
    // Declare a runtime for the async function
    let rt = Runtime::new().unwrap();

    // Declare a group to hold our Throughput benchmarks
    let mut group = c.benchmark_group("Pack Throughput");
    // Associate our input directory size with the benchmark group throughput
    group.throughput(criterion::Throughput::Bytes(input_dir_size));
    // Add the benchmark to the group
    group.bench_function("pack", |b| {
        b.to_async(&rt).iter_batched(
            // Operation needed to make sure pack doesn't fail
            || prep_pack(
                &PathBuf::from(PACKED_DIR),
                &PathBuf::from(MANIFEST_FILE),
            ),
            // The routine to benchmark
            |_| async {
                pack_pipeline(
                    black_box(PathBuf::from(INPUT_DIR)),
                    black_box(PathBuf::from(PACKED_DIR)),
                    black_box(PathBuf::from(MANIFEST_FILE)),
                    black_box(1073741824),
                    black_box(false),
                )
                .await
            },
            // We need to make sure this data is cleared between iterations
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

/// Benchmark unpacking - relies on PACKED_DIR and MANIFEST_FILE being populated
fn unpack_benchmark(c: &mut Criterion) {
    // Figure out how many bytes are in the input directory
    let input_dir_size = dir::get_size(PACKED_DIR).unwrap();
    // Declare a runtime for the async function
    let rt = Runtime::new().unwrap();

    // Declare a group to hold our Throughput benchmarks
    let mut group = c.benchmark_group("Unpack Throughput");
    // Associate our input directory size with the benchmark group throughput
    group.throughput(criterion::Throughput::Bytes(input_dir_size));
    // Add the benchmark to the group
    group.bench_function("unpack", |b| {
        b.to_async(&rt).iter_batched(
            // Operation needed to make sure unpack doesn't fail
            || prep_unpack(
                &PathBuf::from(UNPACKED_DIR),
                &PathBuf::from(MANIFEST_FILE),
            ),
            // The routine to benchmark
            |_| async {
                unpack_pipeline(
                    black_box(PathBuf::from(PACKED_DIR)),
                    black_box(PathBuf::from(UNPACKED_DIR)),
                    black_box(PathBuf::from(MANIFEST_FILE))
                )
                .await
            },
            // We need to make sure this data is cleared between iterations
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

/// Run our end to end pipeline benchmarks sequentially on multiple Input Directories
pub fn pipeline_benchmarks(c: &mut Criterion) {
    pack_benchmark(c);
    unpack_benchmark(c);
}

criterion_group!(benches, pipeline_benchmarks);
criterion_main!(benches);

