#!/bin/bash

#Usage: ./runner.sh --bin <binary-name>

#Function to display usage information
usage(){
    echo "Usage: $0 <binary-name>"
    exit 1
}

# Check if the correct number of arguments is provided
if [ "$#" -ne 1 ]; then
    usage
fi

# Parse arguments
BIN_NAME=$1

# Ensure we are in the correct directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Cargo.toml not found. Please run this script in the
    root directory of your Rust Project."
    exit 1
fi

# Build the project
echo "Building the project..."
cargo build --bin $BIN_NAME

# Check if the build was sucessful
if [ $? -ne 0 ]; then
    echo "Error: Build failed."
    exit 1
fi

# Run the specified binary
echo "Running the binary: $BIN_NAME"
cargo run --bin $BIN_NAME

#Check if the run was successful
if [ $? -ne 0 ]; then
    echo "Error: Failed to run the binary."
    exit 1
fi

echo "Execution finished successfully."


