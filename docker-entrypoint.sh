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
echo "=== Starting extended_connector ==="
set +e
/app/extended_connector 2>&1
EXIT_CODE=$?
echo "=== Bot exited with code: $EXIT_CODE ==="
exit $EXIT_CODE
