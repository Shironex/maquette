#!/usr/bin/env bash
# Sets up the GitHub `release` environment the release workflow signs updates in:
# only `v*` tags may deploy to it, you approve every run, and it holds the updater
# key and its password as environment secrets. Safe to run again.
#
#   scripts/github/setup-release-env.sh [owner/repo] [path/to/updater.key]
#
# Needs the GitHub CLI signed in as the repo owner (`gh auth status`). The key is
# read from the file and the password is asked for; neither is printed.
set -euo pipefail

REPO="${1:-Shironex/maquette}"
KEY_FILE="${2:-$HOME/.maquette-keys/updater.key}"
ENV_NAME="release"

if ! command -v gh >/dev/null 2>&1; then
  echo "The GitHub CLI (gh) is not installed." >&2
  exit 1
fi
if [ ! -s "$KEY_FILE" ]; then
  echo "No key at $KEY_FILE. Pass its path as the second argument." >&2
  exit 1
fi

LOGIN="$(gh api user -q .login)"
USER_ID="$(gh api user -q .id)"
echo "Signed in as $LOGIN, setting up '$ENV_NAME' on $REPO."

# The environment: you as the required reviewer (you may approve your own runs),
# and only the custom deployment policies below may use it.
gh api --method PUT "repos/$REPO/environments/$ENV_NAME" --input - >/dev/null <<JSON || {
{
  "wait_timer": 0,
  "prevent_self_review": false,
  "reviewers": [{ "type": "User", "id": $USER_ID }],
  "deployment_branch_policy": { "protected_branches": false, "custom_branch_policies": true }
}
JSON
  echo "Could not set required reviewers. On a private repo GitHub only allows them on paid plans;" >&2
  echo "creating the environment without a reviewer. Run this again once the repo is public." >&2
  gh api --method PUT "repos/$REPO/environments/$ENV_NAME" --input - >/dev/null <<JSON
{
  "wait_timer": 0,
  "deployment_branch_policy": { "protected_branches": false, "custom_branch_policies": true }
}
JSON
}
echo "Environment '$ENV_NAME' is ready."

# Only tags named v* may deploy to it.
if gh api "repos/$REPO/environments/$ENV_NAME/deployment-branch-policies" \
  -q '.branch_policies[] | select(.type == "tag") | .name' | grep -qx 'v\*'; then
  echo "The v* tag rule is already there."
else
  gh api --method POST "repos/$REPO/environments/$ENV_NAME/deployment-branch-policies" \
    -f name='v*' -f type=tag >/dev/null
  echo "Added the v* tag rule."
fi

# The secrets, read from the file and the prompt, never echoed.
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo "$REPO" --env "$ENV_NAME" <"$KEY_FILE"
echo "Set TAURI_SIGNING_PRIVATE_KEY from $KEY_FILE."

printf 'Password of the updater key (not shown): '
IFS= read -rs KEY_PASSWORD
echo
if [ -z "$KEY_PASSWORD" ]; then
  echo "Empty password, TAURI_SIGNING_PRIVATE_KEY_PASSWORD not set." >&2
  exit 1
fi
printf '%s' "$KEY_PASSWORD" | gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo "$REPO" --env "$ENV_NAME"
unset KEY_PASSWORD
echo "Set TAURI_SIGNING_PRIVATE_KEY_PASSWORD."

echo
echo "Done. Check it at https://github.com/$REPO/settings/environments"
gh secret list --repo "$REPO" --env "$ENV_NAME"
