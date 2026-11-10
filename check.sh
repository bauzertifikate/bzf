#!/bin/sh
# SPDX-License-Identifier: Apache-2.0 OR MIT
# Standalone schema and example validation; optional external fixtures via BZF_DOCUMENTS.
set -eu
cd "$(dirname "$0")/tools"
cargo fmt --all -- --check
cargo test --locked --workspace
