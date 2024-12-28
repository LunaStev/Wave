Write-Host "Initializing and updating submodules..."
git submodule update --init --recursive

Write-Host "Fetching the latest changes for submodules..."
git submodule update --remote --merge

Write-Host "Submodules are up-to-date!"
