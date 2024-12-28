#!/bin/bash

echo "Initializing and updating submodules..."
git submodule update --init --recursive

echo "Fetching the latest changes for submodules..."
git submodule update --remote --merge

echo "Submodules are up-to-date!"
