#!/usr/bin/env bash
# Validate docs link integrity.
#
# Checks:
# 1. All markdown links in crates/, docs/, scripts/, .github/ that reference
#    docs/ resolve to existing files.
# 2. No links reference the work/ scratch directory.
# 3. No absolute filesystem paths appear in docs links.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

if ! command -v rg >/dev/null 2>&1; then
  echo "error: ripgrep (rg) is required" >&2
  exit 2
fi

docs_root="$ROOT/docs"
if [[ ! -d "$docs_root" ]]; then
  echo "error: docs directory not found at $docs_root" >&2
  exit 2
fi

normalize_path() {
  local path="$1"
  local -a parts stack
  IFS='/' read -r -a parts <<< "$path"
  stack=()

  for part in "${parts[@]}"; do
    case "$part" in
      ""|".") continue ;;
      "..")
        if [[ "${#stack[@]}" -gt 0 ]]; then
          unset "stack[${#stack[@]}-1]"
        fi
        ;;
      *)
        stack+=("$part")
        ;;
    esac
  done

  local out=""
  for part in "${stack[@]}"; do
    out+="/$part"
  done
  if [[ -z "$out" ]]; then
    out="/"
  fi
  printf '%s\n' "$out"
}

checked=0
missing=0

# Extract and validate markdown links that target docs/
while IFS= read -r record; do
  [[ -z "$record" ]] && continue

  src_file="${record%%$'\t'*}"
  rest="${record#*$'\t'}"
  src_line="${rest%%$'\t'*}"
  raw_target="${rest#*$'\t'}"

  target="$(printf '%s' "$raw_target" | sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//')"
  target="${target%%[[:space:]]*}"
  if [[ "$target" == \<*\> ]]; then
    target="${target#<}"
    target="${target%>}"
  fi

  [[ -z "$target" ]] && continue
  case "$target" in
    http://*|https://*|mailto:*|\#*) continue ;;
  esac

  path_part="${target%%#*}"
  [[ -z "$path_part" ]] && continue

  if [[ "$path_part" == docs/* ]]; then
    resolved="$(normalize_path "$ROOT/$path_part")"
  elif [[ "$path_part" == /* ]]; then
    continue
  else
    resolved="$(normalize_path "$ROOT/$(dirname "$src_file")/$path_part")"
  fi

  case "$resolved" in
    "$docs_root"/*) ;;
    *) continue ;;
  esac

  checked=$((checked + 1))

  if [[ ! -f "$resolved" ]]; then
    missing=$((missing + 1))
    echo "missing docs link: $src_file:$src_line -> $target (resolved: ${resolved#$ROOT/})"
  fi
done < <(
  # Markdown files in crates/ referencing docs/
  if [[ -d "$ROOT/crates" ]]; then
    while IFS= read -r -d '' file; do
      perl -ne 'while (/\[[^\]]+\]\(([^)]+)\)/g) { print "$ARGV\t$.\t$1\n"; }' "$file"
    done < <(rg -l -0 --pcre2 '\[[^\]]+\]\([^)]*docs/' crates 2>/dev/null || true)
  fi

  # Internal cross-references within docs/
  while IFS= read -r -d '' file; do
    perl -ne 'while (/\[[^\]]+\]\(([^)]+)\)/g) { print "$ARGV\t$.\t$1\n"; }' "$file"
  done < <(rg -l -0 --pcre2 '\[[^\]]+\]\([^)]*\.md' docs 2>/dev/null || true)

  # Markdown files in scripts/ and .github/ referencing docs/
  for dir in scripts .github; do
    if [[ -d "$ROOT/$dir" ]]; then
      while IFS= read -r -d '' file; do
        perl -ne 'while (/\[[^\]]+\]\(([^)]+)\)/g) { print "$ARGV\t$.\t$1\n"; }' "$file"
      done < <(rg -l -0 --pcre2 '\[[^\]]+\]\([^)]*docs/' "$dir" 2>/dev/null || true)
    fi
  done

  # Root markdown files
  for root_file in CLAUDE.md README.md; do
    if [[ -f "$ROOT/$root_file" ]]; then
      perl -ne 'while (/\[[^\]]+\]\(([^)]+)\)/g) { print "$ARGV\t$.\t$1\n"; }' "$root_file"
    fi
  done
)

if [[ "$missing" -gt 0 ]]; then
  echo ""
  echo "checked $checked docs link(s); found $missing missing target(s)"
  exit 1
fi

echo "checked $checked docs link(s); all targets exist"

# Check for links to work/
work_links=0
search_paths=()
for p in docs crates scripts .github CLAUDE.md README.md; do
  [[ -e "$ROOT/$p" ]] && search_paths+=("$p")
done

while IFS= read -r match; do
  [[ -z "$match" ]] && continue
  work_links=$((work_links + 1))
  echo "link to work/ found: $match"
done < <(rg --no-heading -n '\[[^\]]+\]\([^)]*work/' "${search_paths[@]}" 2>/dev/null || true)

if [[ "$work_links" -gt 0 ]]; then
  echo ""
  echo "found $work_links link(s) to work/ directory"
  exit 1
fi

echo "no links to work/ directory found"

# Check for absolute filesystem paths in docs links
abs_path_links=0
while IFS= read -r match; do
  [[ -z "$match" ]] && continue
  abs_path_links=$((abs_path_links + 1))
  echo "absolute path in link: $match"
done < <(rg --no-heading -n '\[[^\]]+\]\(/(?:Users|home|tmp|var|opt|root)/' docs/ 2>/dev/null || true)

if [[ "$abs_path_links" -gt 0 ]]; then
  echo ""
  echo "found $abs_path_links link(s) with absolute filesystem paths in docs/"
  exit 1
fi

echo "no absolute filesystem paths in docs links"
