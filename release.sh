#!/usr/bin/env bash

# Enable debugging output and error handling
set -eo pipefail

# Default values for the variables
new_version=""

# Function to display help information
function show_help {
  echo "Usage: $0 [-n <new_version>]"
  echo
  echo "Options:"
  echo "  -n   Specify the new version of the software or application."
  echo "  -h   Display this help message."
  echo
}

# Parse command line arguments
while getopts "n:h" flag; do
  case "${flag}" in
    n) new_version=${OPTARG} ;;
    h) show_help; exit 0 ;;
    *) echo "Invalid option. Use -h for help." >&2; exit 1 ;;
  esac
done


if [[ -z "${new_version}" ]]; then
  echo "Error: New version (-n) is required. with the following format vX.X.X" >&2
  exit 1
fi

echo "New Version: ${new_version}"


update_version() {
    version="${1#v}"
    if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "Invalid version format: $1"
        exit 1
    fi

    current_version=$(grep -oP '(?<=^version = ")[^"$]*' Cargo.toml)

    if [[ "$current_version" == "$version" ]]; then
        read -p "Are you sure about that? The current version is the same: $current_version. Do you want to proceed? (yes/no): " user_input
        if [[ "$user_input" != "yes" && "$user_input" != "y" ]]; then
            echo "Exiting. No changes were made."
            exit 1
        fi
    fi

    sed -i "s/^version = \".*\"$/version = \"$version\"/" Cargo.toml
    echo "Let's go $version"
}


cargo fmt --all
cargo clippy -- -Dclippy::all -D warnings

update_version "$new_version"

git add --all
git commit --all --signoff --message "release: $new_version"
git tag -a "$new_version" -m "Version $new_version"
git push origin "$new_version"
