#![feature(io_error_more)]
#![feature(buf_read_has_data_left)]
#![deny(unused_crate_dependencies)]

use clap::Parser;
use dataprep_lib::do_pipeline_and_write_metadata::{
    pack_pipeline::pack_pipeline, unpack_pipeline::unpack_pipeline,
};
use fclones::config::{GroupConfig, Parallelism};

mod cli;

#[tokio::main]
async fn main() {
    // Parse command line arguments. see args.rs
    let cli = cli::Args::parse();

    match cli.command {
        cli::Commands::Pack {
            input_dir,
            output_dir,
            manifest_file,
            target_chunk_size,
            follow_links,
        } => {
            let base_dir = input_dir.canonicalize().unwrap();
            println!("base_dir: {:?}", base_dir);
            // we checked over these options manually and sorted them
            let group_config = GroupConfig {
                // will definitely never need to change
                output: None,
                format: Default::default(),
                stdin: false,
                isolate: false, // TODO laudiacay um bug?
                in_place: false,
                no_copy: false,
                rf_over: None,
                rf_under: None,
                unique: false,

                // will probably never need to change
                depth: None,
                match_links: false,
                symbolic_links: false, // TODO laudiacay here be bugs
                transform: None,
                min_size: (0 as usize).into(),
                max_size: None,
                ignore_case: false,
                regex: false,

                // may want to change for feature adds in the future
                hidden: true,
                no_ignore: true, // TODO laudiacay HELPPPP THIS MIGHT BE BUGS
                // TODO laudiacay ????
                name_patterns: vec![".*".into()],
                path_patterns: vec![".*".into()],
                exclude_patterns: vec![],
                hash_fn: Default::default(),
                cache: false,

                // we are using this option it is load bearing
                threads: vec![("default".to_string().parse().unwrap(), Parallelism{random: 1, sequential:1})],
                follow_links,
                base_dir: base_dir.clone().into(),
                paths: vec![base_dir.into()],
            };
            // TODO think about fclones caching for repeated runs :3 this will b useful for backup utility kind of thing
            // TODO groupconfig.threads and think about splitting squential and random io into separate thread pools

            pack_pipeline(
                input_dir,
                output_dir,
                manifest_file,
                target_chunk_size,
                group_config,
            )
            .await
            .unwrap();
        }
        cli::Commands::Unpack {
            input_dir,
            manifest_file,
            output_dir,
        } => {
            unpack_pipeline(input_dir, output_dir, manifest_file)
                .await
                .unwrap();
        }
    }
}
