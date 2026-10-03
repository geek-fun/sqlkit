#!/usr/bin/env bash
#
# Verifies the DuckDB JDBC URL / instance semantics that SQLKit's connection
# form relies on (file auto-creation, read-only, named in-memory instances,
# configuration conflicts). Requires a JDK 11+ on PATH.
#
# Usage: scripts/verify-duckdb-jdbc-urls.sh [driver-version]
#
set -euo pipefail

DRIVER_VERSION="${1:-1.5.6.0}"
GROUP_PATH="org/duckdb/duckdb_jdbc"
JAR_NAME="duckdb_jdbc-${DRIVER_VERSION}.jar"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK_DIR="${TMPDIR:-/tmp}/sqlkit-duckdb-verify"
JAR_PATH="${WORK_DIR}/${JAR_NAME}"

if ! command -v java >/dev/null 2>&1; then
  echo "java is required to run this verification (JDK 11+)" >&2
  exit 1
fi

mkdir -p "${WORK_DIR}"
if [ ! -f "${JAR_PATH}" ]; then
  echo "Downloading ${JAR_NAME} from Maven Central..."
  curl -sSL --max-time 600 \
    -o "${JAR_PATH}" \
    "https://repo1.maven.org/maven2/${GROUP_PATH}/${DRIVER_VERSION}/${JAR_NAME}"
fi

echo "Running DuckDB JDBC checks with driver ${DRIVER_VERSION}"
java -cp "${JAR_PATH}" "${SCRIPT_DIR}/duckdb-jdbc-verify/Verify.java"
