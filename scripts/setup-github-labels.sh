#!/usr/bin/env bash
set -euo pipefail

if ! command -v gh >/dev/null 2>&1; then
  echo "Error: GitHub CLI ('gh') is not installed." >&2
  exit 1
fi

repo="${1:-}"
labels_file="${2:-docs/github-labels.json}"

if [[ -z "${repo}" ]]; then
  echo "Usage: $0 <owner/repo> [labels-json-path]" >&2
  echo "Example: $0 spalencsar/deskify" >&2
  exit 1
fi

if [[ ! -f "${labels_file}" ]]; then
  echo "Error: labels file not found: ${labels_file}" >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "Error: 'jq' is required to read ${labels_file}." >&2
  exit 1
fi

echo "Syncing labels to ${repo} using ${labels_file} ..."

jq -c '.[]' "${labels_file}" | while IFS= read -r label; do
  name="$(jq -r '.name' <<<"${label}")"
  color="$(jq -r '.color' <<<"${label}")"
  description="$(jq -r '.description' <<<"${label}")"

  if gh label edit "${name}" --repo "${repo}" --color "${color}" --description "${description}" >/dev/null 2>&1; then
    echo "Updated: ${name}"
  else
    gh label create "${name}" --repo "${repo}" --color "${color}" --description "${description}" >/dev/null
    echo "Created: ${name}"
  fi
done

echo "Done."
