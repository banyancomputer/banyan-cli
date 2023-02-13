use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use dataprep_pipelines::{
    pipeline::{pack_pipeline::pack_pipeline, unpack_pipeline::unpack_pipeline},
    utils::fs::{
        ensure_path_exists_and_is_dir, ensure_path_exists_and_is_empty_dir, FileStructure,
        FileStructureStrategy,
    },
};
use dir_assert::assert_paths;
use fs_extra::dir;
use lazy_static::lazy_static;
use std::time::Duration;
use std::{env, fs, path::PathBuf};
use test_notifier::TestNotifier;
use tokio::runtime::Runtime;

mod perf;

// Configure the Benching Framework from the Environment -- or use defaults
lazy_static! {
    // Bench set directory configuration
    // Path we will use to hold our benchmarking data
    static ref BENCH_PATH: String = env::var("BENCH_PATH").unwrap_or_else(|_| "bench".to_string());
    // Path we will use to hold inputs we build for benchmarking
    static ref INPUT_PATH: String = env::var("INPUT_PATH").unwrap_or_else(|_| "bench/input".to_string());
    // Path we will use to hold packed data we build from inputs for benchmarking
    static ref PACKED_PATH: String = env::var("PACKED_PATH").unwrap_or_else(|_| "bench/packed".to_string());
    // Path we will use to hold unpacked data we build from packed data for benchmarking
    static ref UNPACKED_PATH: String = env::var("UNPACKED_PATH").unwrap_or_else(|_| "bench/unpacked".to_string());
    // Path we will use to hold the manifest files generated during benchmarking
    static ref MANIFEST_PATH: String = env::var("MANIFEST_PATH").unwrap_or_else(|_| "bench/manifest".to_string());

    // TODO (make this work) IFTTT key to use for sending notifications
    static ref IFTTT_KEY: String = env::var("IFTTT_TEST_WEBHOOK_KEY").unwrap_or_else(|_| "none".to_string());

    // Test Set Generation configuration
    // Defaults to a simple 4x4 file structure with 1Mb of data
    // What sort of File Structures to generate --options [ simple, skinny, wide, file ]
    static ref BENCH_FILE_STRUCTURES_STRING: String = env::var("BENCH_FILE_STRUCTURES").unwrap_or_else(|_| "simple".to_string());
    // How big each test should be (in bytes) Try to use powers of 2 here
    static ref BENCH_FILE_STRUCTURES_SIZE: usize = env::var("BENCH_FILE_STRUCTURES_SIZE")
        .unwrap_or_else(|_| "1048576".to_string()) // Default to 1Mb
        .parse::<usize>()
        .unwrap();
    // How wide a file structure can be
    static ref BENCH_FILE_STRUCTURES_MAX_WIDTH : usize = env::var("BENCH_FILE_STRUCTURES_MAX_WIDTH")
        .unwrap_or_else(|_| "4".to_string()) // Default to 4
        .parse::<usize>()
        .unwrap();
    static ref BENCH_FILE_STRUCTURES_MAX_DEPTH : usize = env::var("BENCH_FILE_STRUCTURES_MAX_DEPTH")
        .unwrap_or_else(|_| "4".to_string()) // Default to 4
        .parse::<usize>()
        .unwrap();

    // Criterion Configuration
    // Defaults to 10 samples per file structure, 30 seconds per sample, 5 seconds of warmup
    // How long to run each sample for
    static ref SAMPLE_TIME: usize = env::var("BENCH_SAMPLE_TIME")
        .unwrap_or_else(|_| "30".to_string()) // Default to 30 seconds
        .parse::<usize>()
        .unwrap();
    // How many samples to draw per File Structure per step
    static ref SAMPLE_SIZE: usize = env::var("BENCH_SAMPLE_COUNT")
        .unwrap_or_else(|_| "10".to_string()) // Default to 10 samples
        .parse::<usize>()
        .unwrap();
    // How long to warmup for before starting to sample
    static ref WARMUP_TIME: usize = env::var("BENCH_WARMUP_TIME")
        .unwrap_or_else(|_| "5".to_string()) // Default to 5 seconds
        .parse::<usize>()
        .unwrap();

    // Correctness Check Configuration
    // Defaults to running a correctness check on the unpacked data - maybe don't use for large tests
    // This should be fine for the default configuration
    // Whether not to run a correctness check on the unpacked data
    static ref DO_CORRECTNESS_CHECK: bool = env::var("BENCH_DO_CORRECTNESS_CHECK")
        .unwrap_or_else(|_| "true".to_string()) // Default to true
        .parse::<bool>()
        .unwrap();
}

// /// Read what File structures we want to test from the environment
// /// # Returns
// /// A list of file structures to test - if nothing is specified we test one
// /// And how big each test should be
// /// Usage:
// ///  BENCH_FILE_STRUCTURES="simple,skinny,wide,file" cargo bench
fn get_desired_file_structures() -> Vec<FileStructure> {
    // Initialize a list of file structures to test
    let mut desired_structures: Vec<FileStructure> = Vec::new();
    // Get the list of file structures to test from the environment
    let bench_file_structures = BENCH_FILE_STRUCTURES_STRING.as_str();
    // Get the size of the test from the environment, as a usize
    let bench_file_structure_size = *BENCH_FILE_STRUCTURES_SIZE;
    // Split the list of file structures into a list of strings
    let file_structures = bench_file_structures.split(",");
    // Get the max width of the file structures
    let max_width = *BENCH_FILE_STRUCTURES_MAX_WIDTH;
    // Get the max depth of the file structures
    let max_depth = *BENCH_FILE_STRUCTURES_MAX_DEPTH;

    println!("Declaring desired file structures:");
    // Print the file structures we are testing
    println!("-> Structures: {}", bench_file_structures);
    // Print the size of the test
    println!("-> Size: {}", bench_file_structure_size);
    // Print the max width of the file structures
    println!("-> Max Width: {}", max_width);
    // Print the max depth of the file structures
    println!("-> Max Depth: {}", max_depth);

    // We're only gonna make balanced trees for now
    let strategy = FileStructureStrategy::Balanced;
    // Iterate through the list of file structures
    for file_structure in file_structures {
        // Add the file structure to our list of desired structures
        match file_structure {
            "skinny" => {
                let structure = FileStructure::new(
                    max_width / 2,
                    max_depth,
                    bench_file_structure_size,
                    strategy.clone(),
                );
                desired_structures.push(structure);
            }
            "wide" => {
                let structure = FileStructure::new(
                    max_width,
                    max_depth / 2,
                    bench_file_structure_size,
                    strategy.clone(),
                );
                desired_structures.push(structure);
            }
            "file" => {
                let structure =
                    FileStructure::new(0, 0, bench_file_structure_size, strategy.clone());
                desired_structures.push(structure);
            }
            // catches simple and anything else
            _ => {
                let structure = FileStructure::new(
                    max_width,
                    max_depth,
                    bench_file_structure_size,
                    strategy.clone(),
                );
                desired_structures.push(structure);
            }
        }
        // Add the structure to the list of desired structures
    }
    // Return the list of desired structures
    desired_structures
}

/// Populate the input directory with a with a test set to benchmark against
/// # Arguments
/// # `desired_structures` - A list of file structures to generate
fn populate_input_dirs(desired_structures: Vec<FileStructure>) {
    println!("Populating inputs...");
    // Get the string from the IFTTT key
    let ifttt_key = IFTTT_KEY.as_str();
    // If the key is not "none", then we want to send notifications
    if ifttt_key != "none" {
        println!("Sending notifications to IFTTT webhook: {}", ifttt_key);
        // Create a notifier
        let _tn = TestNotifier::new_with_message("Populating Input Directories".to_string());
    }
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
    println!("Setting up benchmarking directories...");
    println!("-> Bench Path: {}", BENCH_PATH.as_str());
    println!("-> Input Path: {}", INPUT_PATH.as_str());
    println!("-> Packed Path: {}", PACKED_PATH.as_str());
    println!("-> Unpacked Path: {}", UNPACKED_PATH.as_str());

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
/// // At this point there should be packed and unpacked directories for each file structure
fn cleanup_bench() {
    println!("Cleaning up benchmarking directories...");
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
    packed_path: &PathBuf,
    unpacked_path: &PathBuf,
    manifest_path: &PathBuf,
) {
    // Get the filename of the input directory
    let input_name = unpacked_path.file_name().unwrap().to_str().unwrap();
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
}

/// Run our end to end pipeline benchmarks sequentially on multiple Input Directories
pub fn pipeline_benchmark(c: &mut Criterion) {
    // Define the file structure to test
    let desired_structures = get_desired_file_structures();
    // Setup the bench
    setup_bench(desired_structures);

    // Paths we will not have to mutate
    let root_input_path = PathBuf::from(INPUT_PATH.as_str());
    let packed_path = PathBuf::from(PACKED_PATH.as_str());

    // Read the input directory for the benchmark
    let input_dir = fs::read_dir(root_input_path).unwrap();

    // Get the string from the IFTTT key
    let ifttt_key = IFTTT_KEY.as_str();
    // If the key is not "none", then we want to send notifications
    if ifttt_key != "none" {
        println!("Sending notifications to IFTTT webhook: {}", ifttt_key);
        // Create a notifier
        let _tn = TestNotifier::new_with_message("Running Benchmarks".to_string());
    }

    // Iterate over our input directories and run the benchmarks
    for entry in input_dir {
        // Paths we will have to mutate
        let mut input_path = PathBuf::from(INPUT_PATH.as_str());
        let mut unpacked_path = PathBuf::from(UNPACKED_PATH.as_str());
        let mut manifest_path = PathBuf::from(MANIFEST_PATH.as_str());

        // Get the names of the input entry
        let entry_name = entry.unwrap().file_name();
        // Mutate the input path so we can use it in the benchmark
        input_path.push(entry_name.clone());
        // Mutate the unpacked path so we can use it in the benchmark
        unpacked_path.push(entry_name.clone());
        // Mutate the manifest path so we can use it in the benchmark. Append .json to the end
        manifest_path.push(entry_name.clone());
        manifest_path.set_extension("json");

        // Run the pack benchmark
        pack_benchmark(c, &input_path, &packed_path, &manifest_path);

        // Run the unpack benchmark
        unpack_benchmark(c, &packed_path, &unpacked_path, &manifest_path);

        // If we have correctness testing enabled, run the correctness tests
        if *DO_CORRECTNESS_CHECK {
            println!(
                "Running correctness tests on {}",
                entry_name.to_str().unwrap()
            );
            // Make sure they have the same contents
            assert_paths!(input_path, unpacked_path);
        }
        println!("Finished benchmarking {}", entry_name.to_str().unwrap());
        // Cleanup the bench
        cleanup_bench();
    }
}

fn custom_config() -> Criterion {
    // Get the size of the input directory
    Criterion::default()
        .sample_size(*SAMPLE_SIZE)
        .measurement_time(Duration::from_secs(*SAMPLE_TIME as u64))
        .warm_up_time(Duration::from_secs(*WARMUP_TIME as u64))
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
