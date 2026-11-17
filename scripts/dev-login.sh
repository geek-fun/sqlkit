#!/usr/bin/env bash
#
# Injects a login into `npm run tauri dev` — the debug app cannot receive
# `sqlkit://` deep links (schemes are registered per app bundle) and its
# WebView localStorage is origin-partitioned away from the installed app,
# so the console handoff can never reach it. This script bridges that gap
# via the backend's debug-only injection hook (src-tauri/src/lib.rs).
#
# Two sources, sharing one injection mechanism:
#
#   1. Production app (default) — reads the installed app's WebView
#      localStorage and targets the production console API. Fast path for
#      verifying the real login -> subscription -> entitlement chain.
#
#        scripts/dev-login.sh
#
#   2. Local stack (paste mode) — takes a `sqlkit://auth?…` link copied from
#      the console handoff dialog ("Copy Link" button) and targets the local
#      console stack. Full-chain check against your own backend.
#
#        # in the geekfun repo:  scripts/dev-stack.sh
#        # register/login on http://localhost:5174, handoff dialog -> Copy Link
#        scripts/dev-login.sh --api http://localhost:3000 '<sqlkit://auth?…>'
#
# Tokens are never printed; they live only in the child process environment.
#
# Flags:
#   --api <url>          Console API base (default: https://geekfun-api.wentsen.com;
#                        SQLKIT_CONSOLE_API_URL still overrides this flag)
#   --print-env          write a sourceable env file instead of launching;
#                        path via DEV_LOGIN_ENV_FILE (default /tmp/sqlkit-dev-login.env)

set -euo pipefail
cd "$(dirname "$0")/.."

WEBKIT_ROOT="$HOME/Library/WebKit/club.geekfun.sqlkit"
PASTED_URL=""
API_URL="${SQLKIT_CONSOLE_API_URL:-}"
ENV_FILE=""

while [ $# -gt 0 ]; do
  case "$1" in
    --api)
      API_URL="$2"
      shift 2
      ;;
    --print-env)
      ENV_FILE="${DEV_LOGIN_ENV_FILE:-/tmp/sqlkit-dev-login.env}"
      shift
      ;;
    sqlkit://*)
      PASTED_URL="$1"
      shift
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

# Resolve the auth payload: a pasted handoff link wins; otherwise extract the
# persisted login from the production app's WebView storage.
if [ -n "$PASTED_URL" ]; then
  # parse_auth_from_url (lib.rs) requires token/username/email — check here
  # so a truncated paste fails loudly instead of logging the app out.
  for field in token username email; do
    echo "$PASTED_URL" | grep -q "[?&]$field=" || {
      echo "error: pasted link is missing the required '$field' parameter" >&2
      exit 1
    }
  done
  PAYLOAD="${PASTED_URL#sqlkit://auth?}"
else
  if [ ! -d "$WEBKIT_ROOT" ]; then
    echo "error: $WEBKIT_ROOT not found — install and sign in to the production app first," >&2
    echo "       or paste a 'sqlkit://auth?…' link from the console handoff dialog" >&2
    exit 1
  fi

  # Extract the persisted account JSON (WebKit stores localStorage values as
  # UTF-16LE/BE text; older builds prefix a flag byte) and URL-encode the
  # auth payload fields. The token never reaches stdout.
  PAYLOAD=$(python3 - "$WEBKIT_ROOT" <<'PY'
import glob, json, os, sqlite3, sys
from urllib.parse import quote

root = sys.argv[1]
best = None
for p in glob.glob(os.path.join(root, "**", "LocalStorage", "LocalStorage.sqlite3"), recursive=True):
    try:
        con = sqlite3.connect(f"file:{p}?mode=ro", uri=True)
        row = con.execute("SELECT value FROM ItemTable WHERE key='account'").fetchone()
        con.close()
        if row and (best is None or len(row[0]) > len(best[1])):
            best = row[0]
    except Exception:
        pass

if best is None:
    sys.exit("no persisted account found — sign in to the production app once, then retry")

blob = best
text = None
for enc in ("utf-16-le", "utf-16-be", "utf-8"):
    try:
        cand = blob.decode(enc)
        if cand.lstrip().startswith("{"):
            text = cand
            break
    except Exception:
        continue
if text is None:
    sys.exit("could not decode the persisted account payload")

acct = json.loads(text)
token = acct.get("token") or sys.exit("persisted account has no token")
parts = [
    ("token", token),
    ("username", acct.get("username") or sys.exit("persisted account has no username")),
    ("email", acct.get("email") or sys.exit("persisted account has no email")),
]
if acct.get("userId"):
    parts.append(("userId", acct["userId"]))
if acct.get("avatar"):
    parts.append(("avatar", acct["avatar"]))
print("&".join(f"{k}={quote(str(v), safe='')}" for k, v in parts))
PY
)

  if [ -z "$PAYLOAD" ]; then
    echo "error: failed to build the auth payload" >&2
    exit 1
  fi
fi

export SQLKIT_DEV_AUTH_URL="sqlkit://auth?$PAYLOAD"
export SQLKIT_CONSOLE_API_URL="${API_URL:-https://geekfun-api.wentsen.com}"

echo "[dev-login] SQLKIT_CONSOLE_API_URL=$SQLKIT_CONSOLE_API_URL"
echo "[dev-login] auth payload injected ($(echo "$PAYLOAD" | wc -c | tr -d ' ') bytes)"

if [ "${ENV_FILE:-}" != "" ]; then
  umask 077
  printf 'export SQLKIT_DEV_AUTH_URL=%q\nexport SQLKIT_CONSOLE_API_URL=%q\n' \
    "$SQLKIT_DEV_AUTH_URL" "$SQLKIT_CONSOLE_API_URL" > "$ENV_FILE"
  echo "[dev-login] wrote $ENV_FILE — source it, then run: npm run tauri dev"
else
  exec npm run tauri dev
fi
