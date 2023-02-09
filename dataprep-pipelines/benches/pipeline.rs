use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use dataprep_pipelines::{
    pipeline::{pack_pipeline::pack_pipeline, unpack_pipeline::unpack_pipeline},
    utils::fs::{
        ensure_path_exists_and_is_dir, ensure_path_exists_and_is_empty_dir, FileStructure,
        FileStructureStrategy,
    },
};
use fs_extra::dir;
use lazy_static::lazy_static;
use std::time::Duration;
use std::{env, fs, path::PathBuf};
// use test_notifier::TestNotifier;
use tokio::runtime::Runtime;

mod perf;

// Configure the Benching Framework from the Environment
lazy_static! {
    // Path we will use to hold our benchmarking data
    static ref BENCH_PATH: String = env::var("BENCH_PATH").unwrap_or_else(|_| "bench".to_string());
    // Path we will use to hold inputs we build for benchmarking
    static ref INPUT_PATH: String = env::var("INPUT_PATH").unwrap_or_else(|_| "bench/input".to_string());
    // Path we will use to hold packed data we build from inputs for benchmarking
    static ref PACKED_PATH: String = env::var("PACKED_PATH").unwrap_or_else(|_| "bench/packed".to_string());
    // Path we will use to hold unpacked data we build from packed data for benchmarking
    static ref UNPACKED_PATH: String = env::var("UNPACKED_PATH").unwrap_or_else(|_| "bench/unpacked".to_string());
    // Path we will use to hold the manifest files generated during benchmarking
    static ref MANIFEST_PATH: String = env::var("MANIFESTS_PATH").unwrap_or_else(|_| "bench/manifests".to_string());
    // IFTTT key to use for sending notifications
    static ref IFTTT_KEY: String = env::var("IFTTT_KEY").unwrap_or_else(|_| "none".to_string());
}

/// Populate the input directory with a with a test set to benchmark against
/// # Arguments
/// # `desired_structures` - A list of file structures to generate
pub fn populate_input_dirs(desired_structures: Vec<FileStructure>) {
    // Get the paths we want to use by turning these into a list of paths
    let desired_paths = desired_structures
        .iter()
        .map(|f: &FileStructure| PathBuf::from(INPUT_PATH.as_str()).join(f.to_string()))
        .collect::<Vec<PathBuf>>();

    // Make sure the input directory exists
    ensure_path_exists_and_is_dir(&PathBuf::from(INPUT_PATH.as_str())).unwrap();

    // Clean the input directory of any file structures we don't want to test
    // Get a list of files and directories in the input directory
    let input_dir = PathBuf::from(INPUT_PATH.as_str());
    let input_dir_contents = input_dir.read_dir().unwrap();
    // Iterate through the list of files and directories in the input directory
    for _entry in input_dir_contents {
        let entry = _entry.unwrap();
        // If the path of the entry is not our list of desired paths, remove it
        if !desired_paths.contains(&entry.path()) {
            // If the entry is a directory, remove it recursively
            if entry.path().is_dir() {
                dir::remove(entry.path()).unwrap();
            }
            // If the entry is a file, remove it
            if entry.path().is_file() {
                fs::remove_file(entry.path()).unwrap();
            }
        }
    }

    // Populate the input directory with the desired file structures, skipping any that already exist
    // Iterate through the list of desired paths
    // Keep an index into our list of desired structures
    let mut i = 0;
    for entry in desired_paths {
        // If the path does not exist, then we need to generate the desired files
        if !entry.exists() {
            // Get the desired structure
            let desired_structure = &desired_structures[i];
            // Generate the desired files
            desired_structure.generate(entry).unwrap();
        }
        // Increment the index
        i += 1;
    }
}

/// Setup the benchmarking directories
/// Makes sure output directories exist and are empty
/// Makes sure the input directory exists and is populated with the desired file structures
/// # Arguments
/// desired_structures - The file structures we want to test
fn setup_bench(desired_structures: Vec<FileStructure>) {
    // Make sure the bench directory exists
    ensure_path_exists_and_is_dir(&PathBuf::from(BENCH_PATH.as_str()))
        .map_err(|e| {
            eprintln!("Error creating bench directory: {}", e);
            e
        })
        .unwrap();

    // Make sure the input directory exists
    ensure_path_exists_and_is_dir(&PathBuf::from(INPUT_PATH.as_str()))
        .map_err(|e| {
            eprintln!("Error creating input directory: {}", e);
            e
        })
        .unwrap();
    // Populate the input directory with the desired file structures, as needed
    populate_input_dirs(desired_structures);

    // Make sure the packed directory exists and is empty
    ensure_path_exists_and_is_empty_dir(&PathBuf::from(PACKED_PATH.as_str()), true)
        .map_err(|e| {
            eprintln!("Error creating packed directory: {}", e);
            e
        })
        .unwrap();

    // Make sure the unpacked directory exists and is empty
    ensure_path_exists_and_is_empty_dir(&PathBuf::from(UNPACKED_PATH.as_str()), true)
        .map_err(|e| {
            eprintln!("Error creating unpacked directory: {}", e);
            e
        })
        .unwrap();

    // Make sure the manifest directory exists and is empty
    ensure_path_exists_and_is_empty_dir(&PathBuf::from(MANIFEST_PATH.as_str()), true)
        .map_err(|e| {
            eprintln!("Error creating manifest directory: {}", e);
            e
        })
        .unwrap();
}

// TODO (amiller68): Do we want to correctness check the unpacked files here?
/// Make sure the packed and unpacked directories are empty
fn cleanup_bench() {
    // Make sure the packed directory exists and is empty
    ensure_path_exists_and_is_empty_dir(&PathBuf::from(PACKED_PATH.as_str()), true)
        .map_err(|e| {
            eprintln!("Error creating packed directory: {}", e);
            e
        })
        .unwrap();

    // Make sure the unpacked directory exists and is empty
    ensure_path_exists_and_is_empty_dir(&PathBuf::from(UNPACKED_PATH.as_str()), true)
        .map_err(|e| {
            eprintln!("Error creating unpacked directory: {}", e);
            e
        })
        .unwrap();

    // Make sure the manifest directory exists and is empty
    ensure_path_exists_and_is_empty_dir(&PathBuf::from(MANIFEST_PATH.as_str()), true)
        .map_err(|e| {
            eprintln!("Error creating manifest directory: {}", e);
            e
        })
        .unwrap();
}

/// Make sure packed directory and manifest file are empty for packing
fn prep_pack(packed_path: &PathBuf, manifest_path: &PathBuf) {
    // Ensure the packed directory exists and is empty
    ensure_path_exists_and_is_empty_dir(packed_path, true)
        .map_err(|e| {
            eprintln!("Error creating packed directory: {}", e);
            e
        })
        .unwrap();

    // if the manifest file exists, remove it
    if manifest_path.exists() {
        fs::remove_file(manifest_path).unwrap();
    }
}

/// Make sure the unpacked directory is empty for unpacking and that the manifest file exists
fn prep_unpack(unpacked_path: &PathBuf, manifest_path: &PathBuf) {
    // Ensure the unpacked directory exists and is empty
    ensure_path_exists_and_is_empty_dir(unpacked_path, true)
        .map_err(|e| {
            eprintln!("Error creating unpacked directory: {}", e);
            e
        })
        .unwrap();
    // Make sure the manifest file exists
    assert!(manifest_path.exists());
}

/// Benchmark packing - relies on INPUT_PATH being populated!
/// # Arguments
/// * `c` - Criterion object
/// * `input_path` - Path to the input directory to use for the benchmark. This will change for each benchmark
/// * `packed_path` - Path to the packed directory to use for the benchmark. This will probably be the same as every other benchmark
/// * `manifest_path` - Path to the manifest file to use for the benchmark. This will probably be the same as every other benchmark, until need is demonstrated to keep these.
/// * `result_path` - Path to the results directory to use for the benchmark. This will change for each benchmark
/// * `timestamp` - Timestamp to use for the benchmark
fn pack_benchmark(
    c: &mut Criterion,
    input_path: &PathBuf,
    packed_path: &PathBuf,
    manifest_path: &PathBuf,
) {
    // Get the filename of the input directory
    let input_name = input_path.file_name().unwrap().to_str().unwrap();
    // We use the input_path + timestamp as the benchmark id
    let bench_id = BenchmarkId::new("pack", input_name.to_string());
    // TODO - Might need add check to see if input path has that much data
    // Figure out how many bytes are in the input directory
    let input_dir_size = dir::get_size(input_path).unwrap();
    // Declare a runtime for the async function
    let rt = Runtime::new().unwrap();
    // Declare a group to hold our Throughput benchmarks
    let mut group = c.benchmark_group("Throughput");
    // Associate our input directory size with the benchmark group throughput
    group.throughput(criterion::Throughput::Bytes(input_dir_size));
    // Add the benchmark to the group
    group.bench_function(bench_id, |b| {
        b.to_async(&rt).iter_batched(
            // Operation needed to make sure pack doesn't fail
            || prep_pack(packed_path, manifest_path),
            // The routine to benchmark
            |_| async {
                pack_pipeline(
                    black_box(input_path.clone()),
                    black_box(packed_path.clone()),
                    black_box(manifest_path.clone()),
                    // TODO (amiller68) - make this configurable
                    black_box(1073741824),
                    black_box(false),
                )
                .await
            },
            // We need to make sure this data is cleared between iterations
            // We only want to use one iteration
            BatchSize::PerIteration,
        );
    });
    group.finish();
    // TODO (amiller68) - write results to results_path/pack directory
}

/// Benchmark unpacking - relies on PACKED_DIR and MANIFEST_FILE being populated!
/// # Arguments
/// * `c` - Criterion object
/// * `input_path` - Path to the input directory to use for the benchmark. This will change for each benchmark
/// * `packed_path` - Path to the packed directory to use for the benchmark. This will probably be the same as every other benchmark
/// * `unpacked_path` - Path to the unpacked directory to use for the benchmark. This will probably be the same as every other benchmark
/// * `manifest_path` - Path to the manifest file to use for the benchmark. This will probably be the same as every other benchmark, until need is demonstrated to keep these.
fn unpack_benchmark(
    c: &mut Criterion,
    input_path: &PathBuf,
    packed_path: &PathBuf,
    unpacked_path: &PathBuf,
    manifest_path: &PathBuf,
) {
    // Get the filename of the input directory
    let input_name = input_path.file_name().unwrap().to_str().unwrap();
    // We use the input_path + timestamp as the benchmark id
    let bench_id = BenchmarkId::new("unpack", input_name.to_string());
    // Figure out how many bytes are in the input directory
    let input_dir_size = dir::get_size(packed_path).unwrap();
    // Declare a runtime for the async function
    let rt = Runtime::new().unwrap();

    // Declare a group to hold our Throughput benchmarks
    let mut group = c.benchmark_group("Throughput");
    // Associate our input directory size with the benchmark group throughput
    group.throughput(criterion::Throughput::Bytes(input_dir_size));
    // Add the benchmark to the group
    group.bench_function(bench_id, |b| {
        b.to_async(&rt).iter_batched(
            // Operation needed to make sure unpack doesn't fail
            || prep_unpack(unpacked_path, manifest_path),
            // The routine to benchmark
            |_| async {
                unpack_pipeline(
                    black_box(packed_path.clone()),
                    black_box(unpacked_path.clone()),
                    black_box(manifest_path.clone()),
                )
                .await
            },
            // We need to make sure this data is cleared between iterations
            // We only want to use one iteration
            BatchSize::PerIteration,
        );
    });
    group.finish();
    // TODO (amiller68) - write results to results_path/unpack directory
}

/// Run our end to end pipeline benchmarks sequentially on multiple Input Directories
pub fn pipeline_benchmark(c: &mut Criterion) {
    // TODO - replace with some sort of bench manifest
    // Define the file structure to test
    let desired_structure: FileStructure = FileStructure::new(
        4,                               // width
        4,                               // depth
        1024,                            // target size in bytes (1Kb)
        FileStructureStrategy::Balanced, // Balanced
        true,                            // utf8 only
    );
    // Turn our desired file structure into a one member list
    let desired_structures: Vec<FileStructure> = vec![desired_structure];
    // TODO - ifttt for setting up directory structures
    // Setup the bench
    setup_bench(desired_structures);

    // Paths we will not have to mutate
    let root_input_path = PathBuf::from(INPUT_PATH.as_str());
    let packed_path = PathBuf::from(PACKED_PATH.as_str());
    let unpacked_path = PathBuf::from(UNPACKED_PATH.as_str());

    // TODO - ifttt for notifying when test are done
    // Read the input directory for the benchmark
    let input_dir = fs::read_dir(root_input_path).unwrap();
    // Iterate over our input directories and run the benchmarks
    for entry in input_dir {
        // Paths we will have to mutate
        let mut input_path = PathBuf::from(INPUT_PATH.as_str());
        let mut manifest_path = PathBuf::from(MANIFEST_PATH.as_str());

        // Get the names of the input entry
        let entry_name = entry.unwrap().file_name();
        // Mutate the input path so we can use it in the benchmark
        input_path.push(entry_name.clone());
        // Mutate the manifest path so we can use it in the benchmark. Append .json to the end
        manifest_path.push(entry_name.clone());
        manifest_path.set_extension("json");

        // Run the pack benchmark
        pack_benchmark(c, &input_path, &packed_path, &manifest_path);

        // Run the unpack benchmark
        unpack_benchmark(c, &input_path, &packed_path, &unpacked_path, &manifest_path);
    }
    cleanup_bench();
}

fn custom_config() -> Criterion {
    // Get the size of the input directory
    Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_secs(30))
        .warm_up_time(Duration::from_secs(5))
        .with_profiler(perf::FlamegraphProfiler::new(100))
}

// Wrap our benchmarks in a criterion group
criterion_group! {
    name = benches;
    // Run 10 samples per benchmark -- this is the minimum number of samples you can run
    config = custom_config();
    targets = pipeline_benchmark
}
criterion_main!(benches);
