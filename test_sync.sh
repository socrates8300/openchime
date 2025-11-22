#!/bin/bash

echo "=== Testing OpenChime Sync Issue ==="

# Clean up any existing database
rm -f openchime.db*

echo "1. Starting OpenChime in background..."
timeout 20s cargo run &
APP_PID=$!

# Wait a bit for app to start
sleep 3

echo "2. Adding test account..."
# We'll use a simple approach to test sync
echo "The app should be running now. Let's wait for it to initialize."
sleep 2

echo "3. Checking if database was created..."
if [ -f "openchime.db" ]; then
    echo "✅ Database created"
    echo "Tables:"
    sqlite3 openchime.db ".tables"
    echo ""
    echo "Accounts count:"
    sqlite3 openchime.db "SELECT COUNT(*) FROM accounts;"
    echo "Events count:"
    sqlite3 openchime.db "SELECT COUNT(*) FROM events;"
else
    echo "❌ Database not found"
fi

echo ""
echo "4. Current database contents:"
if [ -f "openchime.db" ]; then
    echo "Accounts:"
    sqlite3 openchime.db "SELECT id, provider, account_name, substr(auth_data, 1, 50) as auth_preview FROM accounts;" 2>/dev/null || echo "No accounts"
    echo ""
    echo "Events:"
    sqlite3 openchime.db "SELECT id, title, substr(start_time, 1, 16) as start FROM events LIMIT 5;" 2>/dev/null || echo "No events"
fi

# Clean up
kill $APP_PID 2>/dev/null
wait $APP_PID 2>/dev/null

echo ""
echo "=== Test completed ==="