#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
#
# Write a release version into Cargo.toml and Cargo.lock, for the release
# workflow. Run from input_viewer_egui/.
#
#   packaging/set-version.sh 0.2.0
#
# Both files, because Cargo.lock records the crate's own version too and
# `--locked` refuses to build when they disagree.
#
# Line endings: on Windows runners the checkout is CRLF, and a pattern
# anchored on a bare \n silently matches nothing. Every pattern here allows
# an optional \r, and the result is checked rather than assumed.
set -euo pipefail

v="${1:?usage: set-version.sh VERSION}"
export V="$v"

# Only the first line-leading `version =` is the package's own: dependency
# versions are inline tables, never at the start of a line.
perl -pi -e 'if (!$done && s/^version = "[^"]*"(\r?)$/version = "$ENV{V}"$1/) { $done = 1 }' Cargo.toml
perl -0pi -e 's/(name = "input_viewer_egui"\r?\nversion = )"[^"]*"/$1"$ENV{V}"/' Cargo.lock

# Check the crate's own entries, not any matching line: some dependency is
# bound to be at 1.0.0.
fail() {
  echo "::error::set-version.sh: $1 does not carry version $v after rewriting" >&2
  exit 1
}
perl -ne '$found //= $1 if /^version = "(.*)"\r?$/; END { exit(!(defined $found && $found eq $ENV{V})) }' Cargo.toml || fail Cargo.toml
perl -0ne 'exit(!/name = "input_viewer_egui"\r?\nversion = "\Q$ENV{V}\E"/)' Cargo.lock || fail Cargo.lock
echo "version set to $v in Cargo.toml and Cargo.lock"
