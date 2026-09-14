#!/usr/bin/env bash

# Validate the repository's single Node version contract before npm or project
# code runs. Callers provide every input; this module owns no mutable state.
dufs_require_exact_node_version() {
  local project_root="$1"
  local required_version="$2"
  local node_command="$3"
  local version_file="$project_root/.node-version"
  local declared_version=""
  local additional_line=""
  local declared_size
  local expected_size
  local node_version_fd
  local node_output
  local displayed_output

  [[ -f "$version_file" && ! -L "$version_file" ]] || {
    printf 'Node version contract is not a physical regular file: %s\n' \
      "$version_file" >&2
    return 1
  }
  declared_size="$(stat -Lc '%s' -- "$version_file")" || {
    printf 'Unable to inspect the Node version contract: %s\n' \
      "$version_file" >&2
    return 1
  }
  expected_size=$((${#required_version} + 1))
  [[ "$declared_size" == "$expected_size" ]] || {
    printf 'The Node version contract must be exactly %s followed by one LF.\n' \
      "$required_version" >&2
    return 1
  }
  exec {node_version_fd}<"$version_file"
  IFS= read -r declared_version <&"$node_version_fd" || {
    exec {node_version_fd}<&-
    printf 'The Node version contract must end with one LF.\n' >&2
    return 1
  }
  if IFS= read -r additional_line <&"$node_version_fd" || \
    [[ -n "$additional_line" ]]
  then
    exec {node_version_fd}<&-
    printf 'The Node version contract must contain exactly one line.\n' >&2
    return 1
  fi
  exec {node_version_fd}<&-
  [[ "$declared_version" == "$required_version" ]] || {
    printf 'Node.js %s is required; .node-version declares %s.\n' \
      "$required_version" \
      "${declared_version:-<empty>}" >&2
    return 1
  }

  node_output="$(
    LC_ALL=C "$node_command" --version || exit $?
    printf '\037'
  )" || {
    printf 'Unable to determine the Node.js version.\n' >&2
    return 1
  }
  if [[ "$node_output" != "v${required_version}"$'\n'$'\037' ]]; then
    displayed_output="${node_output%$'\037'}"
    displayed_output="${displayed_output%$'\n'}"
    printf 'Node.js %s is required; found %q.\n' \
      "$required_version" \
      "${displayed_output:-<empty>}" >&2
    return 1
  fi
}
