<div align="center">
  <a href="https://github.com/banyancomputer/dataprep" target="_blank">
    <img src="https://raw.githubusercontent.com/banyancomputer/dataprep/main/assets/a_logo.png" alt="Banyan Logo" width="100"></img>
  </a>

  <h1 align="center">Dataprep</h1>

  <p>
    <a href="https://crates.io/crates/dataprep">
      <img src="https://img.shields.io/crates/v/dataprep?label=crates" alt="Crate">
    </a>
    <a href="https://codecov.io/gh/banyancomputer/dataprep">
      <img src="https://codecov.io/gh/banyancomputer/dataprep/branch/main/graph/badge.svg?token=SOMETOKEN" alt="Code Coverage"/>
    </a>
    <a href="https://github.com/banyancomputer/dataprep/actions?query=">
      <img src="https://github.com/banyancomputer/dataprep/actions/workflows/tests_and_checks.yml/badge.svg" alt="Build Status">
    </a>
    <a href="https://github.com/banyancomputer/dataprep/blob/main/LICENSE-MIT">
      <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License-MIT">
    </a>
    <a href="https://docs.rs/dataprep">
      <img src="https://img.shields.io/static/v1?label=Docs&message=docs.rs&color=blue" alt="Docs">
    </a>
    <a href="https://discord.gg/aHaSw9zgwV">
      <img src="https://img.shields.io/static/v1?label=Discord&message=join%20us!&color=mediumslateblue" alt="Discord">
    </a>
  </p>
</div>

<div align="center"><sub>:warning: Work in progress :warning:</sub></div>

##

## Outline

- [What is Dataprep?](#dataprep)
- [Installation](#installation)
- [Usage](#usage)
- [Testing the Project](#testing-the-project)
- [Benchmarking the Project](#benchmarking-the-project)
- [Contributing](#contributing)
- [Running dataprep on Docker](#running-dataprep-on-docker)
- [Getting Help](#getting-help)
- [External Resources](#external-resources)
- [License](#license)


## What is Dataprep?
Dataprep is a tool for encrypting and decrypting large files and directories.
It is meant to be used as a tool for preparing large academic and enterprise datasets for storage on
decentralized storage networks such as Filecoin.

## Installation

### Using `cargo`

To install our CLI tool using `cargo`, run:
```console
cargo install --path dataprep
```

[//]: # (TODO: Add more installation instructions here as we add more ways to install the project.)

## Usage
Dataprep is easy to use. To encrypt a directory, run:
```console
dataprep pack --input-dir <INPUT_DIR> --output-dir <PACKED_DIR> --manifest-file <MANIFEST_FILE>
```
Where `<INPUT_DIR>` is the directory you want to encrypt, `<PACKED_DIR>` is the directory you want to store the encrypted files in, and `<MANIFEST_FILE>` is the file you want to store the manifest in.
The manifest file is a JSON file that contains the metadata for the encrypted files, including the file names, keys, and how to inflate the packed files back to the original files.

To decrypt the same directory, run:
```console
dataprep unpack --input-dir <PACKED_DIR>  --output-dir <UNPACKED_DIR> --manifest-file <MANIFEST_FILE>
```
`<UNPACKED_DIR>` will contain the original files from `<INPUT_DIR>` once the process is complete.

## Testing the Project

- Run tests

  ```console
  cargo test
  ```
  This should run all the tests in the workspace.

## Benchmarking the Project

For benchmarking and measuring performance, this project leverages
[criterion][criterion] and a tool called [fake-file][fake-file]. So far these benchmarks
are very simple and only measure the performance of the whole encryption and decryption pipeline.
As such, they alone are not helpful for identifying performance bottlenecks, but can offer the developer a
good baseline for measuring the performance of the project on there local machine.

### Configuring the benchmarks
The one benchmark in this workspace can be configured by editing and sourcing `env/env.benchmark`. See this file 
for more information on how to best configure the benchmarks, as well as info on default values.

### Running the benchmarks
- Run benchmarks

  ```console
  cargo bench 
  ```
  
### Profiling the benchmarks + cli
At the moment, profiling is not built into the benchmarks. However, we do support and recommend using the `flamegraph` crate
for profiling the benchmarks and the CLI. 
It is unclear whether this provides accurate readings with our Async code, but it is a good starting point for profiling.
Use this if you are trying to figure out where bottlenecks exist in `dataprep-lib`.

Example of profiling the benchmarks:
```console 
cargo flamegraph --bench pipeline
```
See the [flamegraph crate][flamegraph] Github page for more information on how to use this tool,
and resources on interpreting the results.

[//]: # (TODO: Implement Docker for this project.)
[//]: # (## Running dataprep on Docker)

[//]: # ()
[//]: # (We recommend setting your [Docker Engine][docker-engine] configuration)

[//]: # (with `experimental` and `buildkit` set to `true`, for example:)

[//]: # ()
[//]: # (``` json)

[//]: # ({)

[//]: # (  "builder": {)

[//]: # (    "gc": {)

[//]: # (      "defaultKeepStorage": "20GB",)

[//]: # (      "enabled": true)

[//]: # (    })

[//]: # (  },)

[//]: # (  "experimental": true,)

[//]: # (  "features": {)

[//]: # (    "buildkit": true)

[//]: # (  })

[//]: # (})

[//]: # (```)

[//]: # ()
[//]: # (- Build a multi-plaform Docker image via [buildx][buildx]:)

[//]: # ()
[//]: # (  ```console)

[//]: # (  docker buildx build --platform=linux/amd64,linux/arm64 -t dataprep --progress=plain .)

[//]: # (  ```)

[//]: # ()
[//]: # (- Run a Docker image &#40;depending on your platform&#41;:)

[//]: # ()
[//]: # (  ```console)

[//]: # (  docker run --platform=linux/amd64 -t dataprep)

[//]: # (  ```)

## Contributing

:balloon: We're thankful for any feedback and help in improving our project!
We have a [contributing guide](./CONTRIBUTING.md) to help you get involved. We
also adhere to our [Code of Conduct](./CODE_OF_CONDUCT.md).

[//]: # (TODO: Implement a Nix flake for this project.)
[//]: # (_### Nix)

[//]: # ()
[//]: # (This repository contains a [Nix flake][nix-flake] that initiates both the Rust)

[//]: # (toolchain set in [rust-toolchain.toml]&#40;./rust-toolchain.toml&#41; and a)

[//]: # ([pre-commit hook]&#40;#pre-commit-hook&#41;. It also installs helpful cargo binaries for)

[//]: # (development. Please install [nix][nix] and [direnv][direnv] to get started.)

[//]: # ()
[//]: # (Run `nix develop` or `direnv allow` to load the `devShell` flake output,)

[//]: # (according to your preference._)

### Formatting

For formatting Rust in particular, please use `cargo fmt` as it uses
specific nightly features we recommend by default.

### Pre-commit Hook

This project recommends using [pre-commit][pre-commit] for running pre-commit

hooks. Please run this before every commit and/or push.

- Install pre-commit on this project

  ```console
  pre-commit install
  ```

- If you are doing interim commits locally, and for some reason if you _don't_

  want pre-commit hooks to fire, you can run

  `git commit -a -m "Your message here" --no-verify`.

### Recommended Development Flow

- We recommend leveraging [cargo-watch][cargo-watch],
  [cargo-expand][cargo-expand] and [irust][irust] for Rust development.
- We recommend using [cargo-udeps][cargo-udeps] for removing unused dependencies
  before commits and pull-requests.

### Conventional Commits

This project *lightly* follows the [Conventional Commits
convention][commit-spec-site] to help explain
commit history and tie in with our release process. The full specification
can be found [here][commit-spec]. We recommend prefixing your commits with
a type of `fix`, `feat`, `docs`, `ci`, `refactor`, etc..., structured like so:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

## Getting Help

For usage questions, usecases, or issues reach out to us in our [Discord channel](https://discord.gg/aHaSw9zgwV).

We would be happy to try to answer your question or try opening a new issue on Github.

## External Resources

These are references to specifications, talks and presentations, etc.

## License


[buildx]: https://github.com/docker/buildx
[cargo-expand]: https://github.com/dtolnay/cargo-expand
[cargo-udeps]: https://github.com/est31/cargo-udeps
[cargo-watch]: https://github.com/watchexec/cargo-watch
[commit-spec]: https://www.conventionalcommits.org/en/v1.0.0/#specification
[commit-spec-site]: https://www.conventionalcommits.org/
[criterion]: https://github.com/bheisler/criterion.rs
[fake-file]: https://crates.io/crates/fake-file
[docker-engine]: https://docs.docker.com/engine/
[direnv]:https://direnv.net/
[irust]: https://github.com/sigmaSd/IRust
[nix]:https://nixos.org/download.html
[nix-flake]: https://nixos.wiki/wiki/Flakes
[pre-commit]: https://pre-commit.com/
[proptest]: https://github.com/proptest-rs/proptest
[strategies]: https://docs.rs/proptest/latest/proptest/strategy/trait.Strategy.html
[flamegraph]: https://github.com/flamegraph-rs/flamegraph
