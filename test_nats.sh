#!/bin/bash

# Test script for NATS integration with Planter

echo "🚀 Starting NATS integration test..."

# Check if NATS server is running
if ! nc -z localhost 4222 2>/dev/null; then
    echo "❌ NATS server not running on localhost:4222"
    echo "Please start NATS server: docker run -p 4222:4222 nats:latest"
    exit 1
fi

echo "✅ NATS server is running"

# Start the Python mock peer in background
echo "🐍 Starting Python mock peer..."
export NATS_URL="nats://localhost:4222"
python3 protocol_peer.py &
PEER_PID=$!

# Give it time to connect
sleep 2

# Start Planter in background
echo "🌱 Starting Planter..."
export NATS_URL="nats://localhost:4222"
export RUST_LOG=info
cargo run &
PLANTER_PID=$!

# Give Planter time to start
sleep 3

# Send a test plan
echo "📤 Sending test plan..."
curl -X POST http://localhost:3030/plan \
  -H "Content-Type: application/json" \
  -d '[
    {
      "kind": "Phase",
      "id": "setup",
      "spec": {
        "description": "Initialize system",
        "selector": {
          "matchLabels": {
            "phase": "setup"
          }
        }
      }
    },
    {
      "kind": "Phase", 
      "id": "deploy",
      "spec": {
        "description": "Deploy application",
        "selector": {
          "matchLabels": {
            "phase": "deploy"
          }
        }
      }
    }
  ]'

echo ""
echo "🔍 Check the logs above for NATS message exchanges"
echo "📋 You should see:"
echo "   - Planter connecting to NATS"
echo "   - Session creation with sessionId"
echo "   - Python peer receiving start message"
echo "   - Python peer sending state/log updates"

# Wait a bit to see the message flow
sleep 5

# Cleanup
echo "🧹 Cleaning up..."
kill $PEER_PID 2>/dev/null
kill $PLANTER_PID 2>/dev/null

echo "✅ Test complete!"
