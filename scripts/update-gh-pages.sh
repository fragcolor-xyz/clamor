#!/usr/bin/env bash

# `set -e`: Abort the script if a command returns with a non-zero exit code.
# `set -u`: Treat unset variables as an error when substituting.
set -eu  

REPO_URL="https://x-access-token:${GITHUB_TOKEN}@github.com/${GITHUB_REPOSITORY}.git"
REMOTE_NAME="origin"
MAIN_BRANCH="devel"
TARGET_BRANCH="gh-pages"
DOC_FOLDER_MAIN_BRANCH="target/doc"
DOC_FOLDER_TARGET_BRANCH="doc"
DOC_INDEX_PAGE="${CARGO_PACKAGE_NAME}/index.html"

cd "$GITHUB_WORKSPACE"
git config user.name "$BOT_NAME"
git config user.email "$BOT_EMAIL"

cargo doc --no-deps # saves doc in $DOC_FOLDER_MAIN_BRANCH

git fetch
# We use the `--force` flag because sometimes `cargo doc` causes the Cargo.lock to be altered. So if we don't use `--force`, we can't checkout the branch `$TARGET_BRANCH`.
git checkout --force "$TARGET_BRANCH" # What the `--force` flag does: "When switching branches, proceed even if the index or the working tree differs from HEAD. This is used to throw away local changes."

rm -rf "${DOC_FOLDER_TARGET_BRANCH}" # because of the `-f` flag, no errors will be outputted if the folder doesn't exist
cp -r "${DOC_FOLDER_MAIN_BRANCH}/." "${DOC_FOLDER_TARGET_BRANCH}" 
echo "<meta http-equiv=refresh content=0;url=${DOC_FOLDER_TARGET_BRANCH}/${DOC_INDEX_PAGE}>" > "index.html"

git add "${DOC_FOLDER_TARGET_BRANCH}"
git add "index.html"

git commit -m "Updated GitHub Pages"
if [ $? -ne 0 ]; then
    echo "nothing to commit"
    exit 0
fi

git remote set-url "$REMOTE_NAME" "$REPO_URL" 
git push --force-with-lease "$REMOTE_NAME" "$TARGET_BRANCH"
