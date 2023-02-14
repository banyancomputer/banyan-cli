use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use dataprep_pipelines::utils::fs::{
    ensure_path_exists_and_is_dir, ensure_path_exists_and_is_empty_dir, FileStructure,
    FileStructureStrategy,
};
use lazy_static::lazy_static;
use std::{env, fs, path::PathBuf, time};

// Configure the Benching Framework from the Environment
lazy_static! {
    // Path we will use to hold our benchmarking data
    static ref BENCH_PATH: String = env::var("BENCH_PATH").unwrap_or_else(|_| "bench".to_string());
    // Path we will use to hold inputs we build for benchmarking
    static ref INPUT_PATH: String = env::var("FILE_BENCH_PATH").unwrap_or_else(|_| "bench/file_structs".to_string());
    static ref IFTTT_KEY: String = env::var("IFTTT_KEY").unwrap_or_else(|_| "none".to_string());
}

/// Ready a directory for benchmarking
#[doc(hidden)]
fn prep_generate(path: &PathBuf) {
    // If the path exists, remove it, regardless of whether it is a file or directory
    if path.exists() {
        fs::remove_dir_all(path).unwrap();
    }
}

/// Bench the generation of a file structure
#[doc(hidden)]
fn balanced_file_structure(c: &mut Criterion) {
    // Get the Bench path and make sure it exists
    let bench_path = PathBuf::from(BENCH_PATH.as_str());
    ensure_path_exists_and_is_dir(&bench_path).unwrap();
    // Get the input path and make sure it exists and is empty
    let balanced_path = PathBuf::from(INPUT_PATH.as_str()).join("balanced");
    ensure_path_exists_and_is_empty_dir(&balanced_path.clone(), true).unwrap();
    // Declare a balanced file structure
    let file_structure = FileStructure::new(
        4,                               // width
        4,                               // depth
        1024 * 1024,                     // target size in bytes (1Mb)
        FileStructureStrategy::Balanced, // Balanced
    );
    // Get a path to the file structure
    let file_structure_path = file_structure.to_path_string();
    // Get the full path to the file structure
    let full_path = balanced_path.join(file_structure_path);
    // Create the benchmark group
    let mut group = c.benchmark_group("balanced_file_structure");
    // Add a throughput benchmark
    group.throughput(criterion::Throughput::Bytes(
        file_structure.target_size as u64,
    ));
    group.bench_function("balanced_file_structure", |b| {
        b.iter_batched(
            || {
                // Prep the input path
                prep_generate(&full_path.clone());
            },
            |_| {
                // Generate the file structure
                file_structure
                    .generate(black_box(full_path.clone()))
                    .unwrap();
            },
            BatchSize::PerIteration,
        )
    });
    // Finish the benchmark group
    group.finish()
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).measurement_time(time::Duration::from_secs(30));
    targets = balanced_file_structure
}
criterion_main!(benches);
