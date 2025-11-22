#!/bin/bash

echo "=== Testing OpenChime Full Sync Flow ==="

# Clean up any existing database
rm -f openchime.db*

echo "1. Starting OpenChime in background..."
cargo run &
APP_PID=$!

# Wait for app to fully start
echo "2. Waiting for app to initialize (5 seconds)..."
sleep 5

echo "3. Checking initial database state..."
if [ -f "openchime.db" ]; then
    echo "✅ Database created"
    echo "Initial accounts count: $(sqlite3 openchime.db 'SELECT COUNT(*) FROM accounts;')"
    echo "Initial events count: $(sqlite3 openchime.db 'SELECT COUNT(*) FROM events;')"
else
    echo "❌ Database not found"
    exit 1
fi

echo ""
echo "4. Now you need to manually:"
echo "   - Open the app (it should be running)"
echo "   - Go to Settings tab"
echo "   - Enter 'Test Account' in Account Name field"
echo "   - Click 'Use Sample Holidays' button"
echo "   - Click 'Add Account' button"
echo "   - Wait for account to appear in list"
echo "   - Go to Calendar tab"
echo "   - Click 'Sync Calendars' button"
echo ""
echo "5. Press Enter when you've done this to check results..."
read

echo ""
echo "6. Checking final database state..."
if [ -f "openchime.db" ]; then
    echo "✅ Database exists"
    echo "Final accounts count: $(sqlite3 openchime.db 'SELECT COUNT(*) FROM accounts;')"
    echo "Final events count: $(sqlite3 openchime.db 'SELECT COUNT(*) FROM events;')"
    
    if [ $(sqlite3 openchime.db 'SELECT COUNT(*) FROM accounts;') -gt 0 ]; then
        echo "✅ Account was added!"
        echo "Account details:"
        sqlite3 openchime.db 'SELECT id, provider, account_name, substr(auth_data, 1, 50) as url FROM accounts;'
        
        if [ $(sqlite3 openchime.db 'SELECT COUNT(*) FROM events;') -gt 0 ]; then
            echo "✅ Events were synced!"
            echo "Recent events:"
            sqlite3 openchime.db 'SELECT title, substr(start_time, 1, 16) as start FROM events ORDER BY start_time DESC LIMIT 5;'
        else
            echo "❌ No events were synced"
        fi
    else
        echo "❌ No account was added"
    fi
else
    echo "❌ Database not found"
fi

# Clean up
kill $APP_PID 2>/dev/null
wait $APP_PID 2>/dev/null

echo ""
echo "=== Test completed ==="