#!/bin/sh
set -e

echo "=== Docker Container Starting ==="
echo "Working directory: $(pwd)"
echo "Files in /app:"
ls -la /app/
echo ""
echo "Environment check:"
echo "- RUST_LOG: ${RUST_LOG:-not set}"
echo "- RUST_BACKTRACE: ${RUST_BACKTRACE:-not set}"
echo ""
echo "Python version:"
python3 --version
echo ""
echo "Testing Python signing script:"
ls -la /app/scripts/sign_order.py
echo ""
echo "Ensuring bot_state.json exists and is writable..."
if [ ! -e "/app/bot_state.json" ]; then
  echo "bot_state.json not found; creating empty state file at /app/bot_state.json"
  echo '{"current_position":null,"last_rotation_time":null,"total_rotations":0}' > /app/bot_state.json || true
fi
chmod 664 /app/bot_state.json 2>/dev/null || true

echo "=== Starting extended_connector ==="
set +e
/app/extended_connector 2>&1
EXIT_CODE=$?
echo "=== Bot exited with code: $EXIT_CODE ==="
exit $EXIT_CODE
