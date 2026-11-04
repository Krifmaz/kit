#!/usr/bin/env bash
# Proves the template (DESIGN.md §13, §17 "a generated app builds, runs, and shows every kit feature"):
#   1. generates an app from template/ (with the Android app) and checks no template placeholder is left;
#   2. points its kit dependencies at this checkout (a copy in <app>/kit-src, so Docker can build it too);
#   3. checks its copies of the kit's stylesheets are the kit's;
#   4. server/: fmt, clippy -D warnings, tests (every kit endpoint);
#   5. web/: wasm32 check and clippy, tests, trunk build;
#   6. runs the server with the web app, asks its health check, and walks every feature in a browser
#      (scripts/template-smoke.cjs, when node and Playwright are there);
#   7. unless NO_DOCKER=1: builds the Docker image, runs it read-only and waits for its health check.
#
#   scripts/template-check.sh [work dir]        (default: a new temporary folder)
# Needs cargo-generate, the wasm32-unknown-unknown target and trunk on PATH.
set -euo pipefail

KIT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="${1:-$(mktemp -d)}"
NAME=kit-demo
TITLE="Kit Demo"
APP="$WORK/$NAME"
step() { echo; echo "== $*"; }

step "Generate $NAME from template/"
rm -rf "$APP"
mkdir -p "$WORK"
(cd "$WORK" && cargo generate --path "$KIT/template" --name "$NAME" --define app_title="$TITLE" --define android=true --silent)
# GitHub's own ${{ … }} in the workflows is expected; any other {{ or {% is a placeholder that was not filled
if grep -rnI --exclude-dir=.git -e '{%' -e '{{' "$APP" | grep -v '\${{'; then
  echo "Template placeholders were left in the generated app (above)." >&2
  exit 1
fi

step "Point the app at this kit checkout"
mkdir -p "$APP/kit-src"
(cd "$KIT" && git ls-files -z --cached --others --exclude-standard | grep -zv '^template/' | xargs -0 cp --parents -t "$APP/kit-src")
python3 "$KIT/scripts/template-local-kit.py" "$APP" "$APP/kit-src"

step "The app's stylesheets are the kit's"
cmp "$KIT/design/out/tokens.css" "$APP/web/static/css/tokens.css"
cmp "$KIT/crates/kit-leptos/style/kit.css" "$APP/web/static/css/kit.css"

step "server/: fmt, clippy, tests"
(cd "$APP/server" && cargo generate-lockfile && cargo fmt --all --check \
  && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked)

step "web/: wasm32 check, clippy, tests, trunk build"
(cd "$APP/web" && cargo generate-lockfile && cargo fmt --all --check \
  && cargo check --locked --target wasm32-unknown-unknown \
  && cargo clippy --locked --target wasm32-unknown-unknown -- -D warnings \
  && cargo test --locked && trunk build --release --locked)

step "Run it and walk every feature"
DATA="$WORK/data"
rm -rf "$DATA"
PORT="${PORT:-18080}"
(cd "$APP/server" && cargo build --locked)
APP_DATA_DIR="$DATA" APP_STATIC_DIR="$APP/web/dist" APP_PORT="$PORT" APP_ADMIN_PASSWORD="kit-demo-password" \
  "$APP/server/target/debug/$NAME" > "$WORK/server.log" 2>&1 &
SERVER=$!
trap 'kill $SERVER 2>/dev/null || true' EXIT
for _ in $(seq 1 60); do APP_PORT="$PORT" "$APP/server/target/debug/$NAME" healthcheck && break; sleep 1; done
APP_PORT="$PORT" "$APP/server/target/debug/$NAME" healthcheck || { cat "$WORK/server.log"; echo "The server never answered its health check." >&2; exit 1; }
if command -v node > /dev/null && node -e "require('playwright')" 2> /dev/null; then
  node "$KIT/scripts/template-smoke.cjs" "http://127.0.0.1:$PORT" admin kit-demo-password "$WORK/screenshots"
else
  echo "Playwright is not installed here: the browser walk is skipped."
fi
kill $SERVER

if [ "${NO_DOCKER:-}" != 1 ]; then
  step "Docker image: build, run read-only, health check"
  # the image builds from the app folder; the kit copy goes in next to server/ and web/
  sed -i 's|^WORKDIR /src$|WORKDIR /src\nCOPY kit-src ./kit-src|' "$APP/Dockerfile"
  docker build -t "$NAME:check" "$APP"
  docker rm -f "$NAME-check" > /dev/null 2>&1 || true
  docker run -d --name "$NAME-check" --read-only -v "$NAME-check-data:/data" -p "$((PORT + 1)):8080" "$NAME:check"
  for _ in $(seq 1 30); do
    status=$(docker inspect -f '{{.State.Health.Status}}' "$NAME-check")
    [ "$status" = healthy ] && break
    sleep 3
  done
  docker logs "$NAME-check" | tail -20
  [ "$(docker inspect -f '{{.State.Health.Status}}' "$NAME-check")" = healthy ] || { echo "The container never became healthy." >&2; exit 1; }
  [ "$(docker inspect -f '{{.Config.User}}' "$NAME:check")" = nonroot ] || { echo "The image does not run as a non-root user." >&2; exit 1; }
  docker rm -f "$NAME-check" > /dev/null
  docker volume rm "$NAME-check-data" > /dev/null
fi

step "The generated app builds, runs and shows every kit feature."
