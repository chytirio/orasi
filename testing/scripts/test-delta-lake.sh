#!/bin/bash

# Delta Lake Testing Script
# This script runs basic tests to validate the Delta Lake setup

set -e

echo "🧪 Running Delta Lake Tests..."

# Check if services are running
echo "🔍 Checking if services are running..."
if ! docker-compose ps | grep -q "Up"; then
    echo "❌ Services are not running. Start them with: docker-compose up -d"
    exit 1
fi

# Test MinIO connectivity
echo "🪣 Testing MinIO connectivity..."
if curl -s http://localhost:9000/minio/health/live > /dev/null; then
    echo "✅ MinIO is accessible"
else
    echo "❌ MinIO is not accessible"
    exit 1
fi

# Test Spark connectivity
echo "⚡ Testing Spark connectivity..."
if curl -s http://localhost:8080 > /dev/null; then
    echo "✅ Spark Master is accessible"
else
    echo "❌ Spark Master is not accessible"
    exit 1
fi

# Run Delta Lake test
echo "🐍 Running Delta Lake Python test..."
docker-compose run --rm delta-test

echo ""
echo "🎉 All tests completed successfully!"
echo ""
echo "📊 Test Results:"
echo "  ✅ MinIO connectivity"
echo "  ✅ Spark connectivity"
echo "  ✅ Delta Lake operations"
echo ""
echo "🚀 You can now:"
echo "  • Open Jupyter Notebook at http://localhost:8888"
echo "  • Access MinIO Console at http://localhost:9001"
echo "  • Monitor Spark at http://localhost:8080"
echo ""
