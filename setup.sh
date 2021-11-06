#!/usr/bin/env bash

echo ">> Installing git hooks..."
./tools/install-hooks
echo ">> Disabling empty \`git add\` warnings.."
git config --local advice.addEmptyPathspec false
echo ">> Done!"
