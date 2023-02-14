# syntax=docker/dockerfile:1
# Docker file for setting up a benchmarking environment for the DataPrep Pipelines
# Use this if you wanna profile the pipelines, but dont have access to perf events
# (such as on a Mac)

# Our build base
FROM rust:1.66 as build

WORKDIR /dataprep

# Install the nightly toolchain
RUN rustup toolchain install nightly
RUN cargo +nightly install cargo-criterion

# Install libssl-dev
RUN apt-get update && apt-get install -y libssl-dev pkg-config linux-perf

# Copy over the workspace
RUN rm -rf /dataprep
# Copy the entire workspace into the container
COPY . /dataprep

# this build step will cache your dependencies
RUN cargo +nightly build --bench pipeline --release

RUN mkdir -p bench/input
RUN mkdir -p bench/packed
RUN mkdir -p bench/unpacked
RUN mkdir -p bench/manifests
RUN mkdir -p bench/results

# TODO: Some sort of environment config for the benchmarking
# For now, defaults are fine

# Echo a run command to the console for normal use
RUN echo "cargo +nightly  criterion --bench pipeline" > run.sh
# Make the run command executable
RUN chmod +x run.sh

# Echo a run profile command to the console for profiling
RUN echo "cargo +nightly bench --bench pipeline  -- --profile-time=15" > run_profile.sh
# Make the run profile command executable
RUN chmod +x run_profile.sh
