#!/usr/bin/env bash

export SHELLOPTS
export RUST_BACKTRACE=full RUST_LOG=info,cfn_node=debug

should_remove_old_state="${REMOVE_OLD_STATE:-}"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
nodes_dir="$(dirname "$script_dir")/nodes"
deploy_dir="$(dirname "$script_dir")/deploy"

# The following environment variables are used in the contract tests.
# We may load all contracts within the following folder to the test environment.
export TESTING_CONTRACTS_DIR="$deploy_dir/contracts"

# Initialize the dev-chain if it does not exist.
# This script is nilpotent, so it is safe to run multiple times.
"$deploy_dir/init-dev-chain.sh"

# Load the environment variables from the .env file generated by create-dotenv-file.sh.
# These are environment variables that specify the contract hashes and other information.
if [[ ! -f "$deploy_dir/.env" ]]; then
    echo "The .env file does not exist. This should no happen" >&2
    echo "In case of issue pesisting, run $deploy_dir/init-dev-chain.sh -f to reintialize the devchain"
    exit 1
else
    if [ -n "$should_remove_old_state" ]; then
        echo "starting to reset ...."
        rm -rf "$nodes_dir"/*/ckb/store
        "$deploy_dir/init-dev-chain.sh" -f
    fi
    export $(xargs <"$deploy_dir/.env")
fi

if [[ -f "$deploy_dir/.env.local" ]]; then
    # Local environment variables, may used to override the default ones.
    export $(xargs <"$deploy_dir/.env.local")
fi

ckb run -C "$deploy_dir/node-data" --indexer &

# Start the dev node in the background.
cd "$nodes_dir" || exit 1

start() {
    cargo run -- "$@"
}

if [ "$#" -ne 1 ]; then
    LOG_SURFFIX=$' [node 1]\n' start -d 1 &
    LOG_SURFFIX=$' [node 2]\n' start -d 2 &
    LOG_SURFFIX=$' [node 3]\n' start -d 3 &
else
    for id in "$@"; do
        LOG_SURFFIX=" [$id]"$'\n' start -d "$id" &
    done
fi

# we will exit when any of the background processes exits.
# we don't use `wait -n` because of compatibility issues between bash and zsh
initial_jobs=$(jobs -p | wc -l)
while true; do
    current_jobs=$(jobs -p | wc -l)
    if [ "$current_jobs" -lt "$initial_jobs" ]; then
        echo "A background job has exited, exiting ..."
        exit 1
    fi
    sleep 1
done