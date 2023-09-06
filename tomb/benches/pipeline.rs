use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use dir_assert::assert_paths;
use fake_file::{
    utils::{ensure_path_exists_and_is_dir, ensure_path_exists_and_is_empty_dir},
    Strategy, Structure,
};
use fs_extra::dir;
use lazy_static::lazy_static;
use log::{error, info};
use std::{
    env, fs,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};
use tokio::runtime::Runtime;
use tomb::{
    cli::command::BucketSpecifier,
    pipelines::{bundle, extract},
    types::config::globalconfig::GlobalConfig,
};

// Configure the Benching Framework from the Environment -- or use defaults
lazy_static! {
    // Bench set directory configuration
    // Path we will use to hold our benchmarking data
    static ref BENCH_PATH: String = env::var("BENCH_PATH").unwrap_or_else(|_| "target/bench".to_string());
    // Path we will use to hold inputs we build for benchmarking
    static ref INPUT_PATH: String = env::var("INPUT_PATH").unwrap_or_else(|_| "target/bench/input".to_string());
    // Path we will use to hold bundleed data we build from inputs for benchmarking
    static ref PACKED_PATH: String = env::var("PACKED_PATH").unwrap_or_else(|_| "target/bench/bundleed".to_string());
    // Path we will use to hold extracted data we build from bundleed data for benchmarking
    static ref UNPACKED_PATH: String = env::var("UNPACKED_PATH").unwrap_or_else(|_| "target/bench/extracted".to_string());

    // Test Set Generation configuration
    // Defaults to a simple 4x4 file structure with 1Mb of data
    // What sort of File Structures to generate --options [ simple, skinny, wide, file ]
    static ref BENCH_FILE_STRUCTURES_STRING: String = env::var("BENCH_FILE_STRUCTURES").unwrap_or_else(|_| "Simple".to_string());
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
        .unwrap_or_else(|_| "1".to_string()) // Default to 30 seconds
        .parse::<usize>()
        .unwrap();
    // How many samples to draw per File Structure per step
    static ref SAMPLE_SIZE: usize = env::var("BENCH_SAMPLE_COUNT")
        .unwrap_or_else(|_| "10".to_string()) // Default to 10 samples
        .parse::<usize>()
        .unwrap();
    // How long to warmup for before starting to sample
    static ref WARMUP_TIME: usize = env::var("BENCH_WARMUP_TIME")
        .unwrap_or_else(|_| "1".to_string()) // Default to 5 seconds
        .parse::<usize>()
        .unwrap();

    // Correctness Check Configuration
    // Defaults to running a correctness check on the extracted data - maybe don't use for large tests
    // This should be fine for the default configuration
    // Whether not to run a correctness check on the extracted data
    static ref DO_CORRECTNESS_CHECK: bool = env::var("BENCH_DO_CORRECTNESS_CHECK")
        .unwrap_or_else(|_| "true".to_string()) // Default to true
        .parse::<bool>()
        .unwrap();
}

/// Read what File structures we want to test from the environment (or use defaults)
/// # Returns
/// A list of file structures to test from the environment
#[doc(hidden)]
fn get_desired_file_structures() -> Vec<Structure> {
    // Initialize a list of file structures to test
    #[allow(unused_mut)]
    let mut desired_structures: Vec<Structure> = Vec::new();
    // Get the list of file structures to test from the environment
    let bench_file_structures = BENCH_FILE_STRUCTURES_STRING.as_str();
    // Get the size of the test from the environment, as a usize
    let bench_file_structure_size = *BENCH_FILE_STRUCTURES_SIZE;
    // Split the list of file structures into a list of strings
    let file_structures = bench_file_structures.split(',');
    // Get the max width of the file structures
    let max_width = *BENCH_FILE_STRUCTURES_MAX_WIDTH;
    // Get the max depth of the file structures
    let max_depth = *BENCH_FILE_STRUCTURES_MAX_DEPTH;

    info!("Declaring desired file structures:");
    //  the file structures we are testing
    info!("-> Structures: {}", bench_file_structures);
    //  the size of the test
    info!("-> Size: {}", bench_file_structure_size);
    //  the max width of the file structures
    info!("-> Max Width: {}", max_width);
    //  the max depth of the file structures
    info!("-> Max Depth: {}", max_depth);

    // Iterate through the list of file structures
    for file_structure in file_structures {
        // Determine what Strategy we want to use
        let strategy = Strategy::from_str(file_structure).unwrap();

        let s = Structure::new(max_width, max_depth, bench_file_structure_size, strategy);
        desired_structures.push(s);
    }
    // Return the list of desired structures
    desired_structures
}

/// Populate the input directory with a with a test set to benchmark against
/// If the input directory already exists, it will be cleaned of any file structures we don't want to test
/// Any pre-existing file structures we want to test will be left alone
#[doc(hidden)]
fn populate_input_dirs() {
    // Get the desired file structures
    let desired_structures = get_desired_file_structures();

    // Get the paths we want to use by turning these into a list of paths
    let desired_paths = desired_structures
        .iter()
        .map(|f: &Structure| PathBuf::from(INPUT_PATH.as_str()).join(f.to_path_string()))
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
    for (i, entry) in desired_paths.into_iter().enumerate() {
        // If the path does not exist, then we need to generate the desired files
        if !entry.exists() {
            // Get the desired structure
            #[allow(unused_mut)]
            let mut desired_structure = &desired_structures[i];
            // Generate the desired files
            desired_structure.generate(&entry).unwrap();
        }
    }
}

/// Setup the benchmarking directories
/// Makes sure output directories exist and are empty
/// Makes sure the input directory exists and is populated with the desired file structures
/// Makes sure the bundleed directory exists and is empty
/// Makes sure the extracted directory exists and is empty
/// Makes sure the manifest file directory exists and is empty
#[doc(hidden)]
fn setup_bench() {
    info!("Setting up benchmarking directories...");
    info!("-> Bench Path: {}", BENCH_PATH.as_str());
    info!("-> Input Path: {}", INPUT_PATH.as_str());
    info!("-> Bundleed Path: {}", PACKED_PATH.as_str());
    info!("-> Extracted Path: {}", UNPACKED_PATH.as_str());

    // Make sure the bench directory exists
    ensure_path_exists_and_is_dir(&PathBuf::from(BENCH_PATH.as_str()))
        .map_err(|e| {
            error!("Error creating bench directory: {}", e);
            e
        })
        .unwrap();

    // Make sure the input directory exists
    ensure_path_exists_and_is_dir(&PathBuf::from(INPUT_PATH.as_str()))
        .map_err(|e| {
            error!("Error creating input directory: {}", e);
            e
        })
        .unwrap();
    // Populate the input directory with the desired file structures, as needed
    populate_input_dirs();

    // Make sure the bundleed directory exists and is empty
    ensure_path_exists_and_is_dir(&PathBuf::from(PACKED_PATH.as_str()))
        .map_err(|e| {
            error!("Error creating bundleed directory: {}", e);
            e
        })
        .unwrap();

    // Make sure the extracted directory exists and is empty
    ensure_path_exists_and_is_dir(&PathBuf::from(UNPACKED_PATH.as_str()))
        .map_err(|e| {
            error!("Error creating extracted directory: {}", e);
            e
        })
        .unwrap();
}

/// Make sure the bundleed and extracted directories are empty, using the environment variables (or defaults) for the paths
#[doc(hidden)]
fn cleanup_bench() {
    info!("Cleaning up benchmarking directories...");
    // Make sure the bundleed directory exists and is empty
    ensure_path_exists_and_is_empty_dir(&PathBuf::from(PACKED_PATH.as_str()), true)
        .map_err(|e| {
            error!("Error creating bundleed directory: {}", e);
            e
        })
        .unwrap();

    // Make sure the extracted directory exists and is empty
    ensure_path_exists_and_is_empty_dir(&PathBuf::from(UNPACKED_PATH.as_str()), true)
        .map_err(|e| {
            error!("Error creating extracted directory: {}", e);
            e
        })
        .unwrap();
}

/// Make sure bundleed directory is empty for bundleing
#[doc(hidden)]
fn prep_bundle(bundleed_path: &Path) {
    // Ensure the bundleed directory exists and is empty
    ensure_path_exists_and_is_empty_dir(bundleed_path, true)
        .map_err(|e| {
            error!("Error creating bundleed directory: {}", e);
            e
        })
        .unwrap();

    // if the manifest file exists, remove it
    let manifest_path = bundleed_path.with_file_name("manifest.json");
    if manifest_path.exists() {
        fs::remove_file(manifest_path).unwrap();
    }
}

/// Make sure the extracted directory is empty for extracting and that the manifest file exists
#[doc(hidden)]
fn prep_extract(extracted_path: &Path) {
    // Ensure the extracted directory exists and is empty
    ensure_path_exists_and_is_empty_dir(extracted_path, true)
        .map_err(|e| {
            error!("Error creating extracted directory: {}", e);
            e
        })
        .unwrap();
}

/// Benchmark bundleing - relies on input_path being populated!
/// # Arguments
/// * `c` - Criterion object
/// * `input_path` - Path to the input directory to use for the benchmark. This will change for each benchmark
/// * `bundleed_path` - Path to the bundleed directory to use for the benchmark. This will probably be the same as every other benchmark
/// * `result_path` - Path to the results directory to use for the benchmark. This will change for each benchmark
/// * `timestamp` - Timestamp to use for the benchmark
fn bundle_benchmark(c: &mut Criterion, input_path: &Path, bundleed_path: &Path) {
    // Get the filename of the input directory
    let input_name = input_path.file_name().unwrap().to_str().unwrap();
    // We use the input_path + timestamp as the benchmark id
    let bench_id = BenchmarkId::new("bundle", input_name.to_string());
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
            // Operation needed to make sure bundle doesn't fail
            || prep_bundle(bundleed_path),
            // The routine to benchmark
            |_| async {
                bundle::pipeline(
                    black_box(&mut GlobalConfig::from_disk().await?),
                    black_box(&BucketSpecifier::with_origin(input_path)),
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

/// Benchmark extracting - relies on PACKED_PATH and MANIFEST_PATH having bundleed data!
/// # Arguments
/// * `c` - Criterion object
/// * `bundleed_path` - Path to the bundleed directory to use for the benchmark. This will probably be the same as every other benchmark
/// * `extracted_path` - Path to the extracted directory to use for the benchmark. This will probably be the same as every other benchmark
/// * `manifest_path` - Path to the manifest file to use for the benchmark. This will probably be the same as every other benchmark, until need is demonstrated to keep these.
fn extract_benchmark(c: &mut Criterion, bundleed_path: &PathBuf, extracted_path: &PathBuf) {
    // Get the filename of the input directory
    let input_name = extracted_path.file_name().unwrap().to_str().unwrap();
    // We use the input_path + timestamp as the benchmark id
    let bench_id = BenchmarkId::new("extract", input_name.to_string());
    // Figure out how many bytes are in the input directory
    let input_dir_size = dir::get_size(bundleed_path).unwrap();
    // Declare a runtime for the async function
    let rt = Runtime::new().unwrap();

    // Declare a group to hold our Throughput benchmarks
    let mut group = c.benchmark_group("Throughput");
    // Associate our input directory size with the benchmark group throughput
    group.throughput(criterion::Throughput::Bytes(input_dir_size));
    // Add the benchmark to the group
    group.bench_function(bench_id, |b| {
        b.to_async(&rt).iter_batched(
            // Operation needed to make sure extract doesn't fail
            || prep_extract(extracted_path),
            // The routine to benchmark
            |_| async {
                extract::pipeline(
                    black_box(&mut GlobalConfig::from_disk().await?),
                    black_box(&BucketSpecifier::with_origin(bundleed_path)),
                    black_box(extracted_path),
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
/// # Arguments
/// * `c` - Criterion object
fn pipeline_benchmark(c: &mut Criterion) {
    // Setup the bench - populate the input directory
    setup_bench();

    // Where we will store our input sets
    let root_input_path = PathBuf::from(INPUT_PATH.as_str());
    // Where we will store our bundleed data
    let bundleed_path = PathBuf::from(PACKED_PATH.as_str());

    // Read the input directory for the benchmark
    let root_input_dir = fs::read_dir(root_input_path).unwrap();

    // Iterate over our input directories and run the benchmarks
    for entry in root_input_dir {
        // Paths we will have to mutate
        let mut input_path = PathBuf::from(INPUT_PATH.as_str());
        let mut extracted_path = PathBuf::from(UNPACKED_PATH.as_str());

        // Get the names of the input entry
        let entry_name = entry.unwrap().file_name();
        // Mutate the input path so we can use it in the benchmark
        input_path.push(entry_name.clone());
        // Mutate the extracted path so we can use it in the benchmark
        extracted_path.push(entry_name.clone());

        // Run the bundle benchmark
        bundle_benchmark(c, &input_path, &bundleed_path);

        // Run the extract benchmark
        extract_benchmark(c, &bundleed_path, &extracted_path);

        // If we have correctness testing enabled, run the correctness tests
        if *DO_CORRECTNESS_CHECK {
            info!(
                "Running correctness tests on {}",
                entry_name.to_str().unwrap()
            );
            // Make sure they have the same contents
            assert_paths!(input_path, extracted_path);
        }
        info!("Finished benchmarking {}", entry_name.to_str().unwrap());
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
}

// Wrap our benchmarks in a criterion group
criterion_group! {
    name = benches;
    // Run 10 samples per benchmark -- this is the minimum number of samples you can run
    config = custom_config();
    targets = pipeline_benchmark
}
criterion_main!(benches);
