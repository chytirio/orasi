#!/bin/bash

# Delta Lake Testing Platform Setup Script
# This script initializes the testing environment for Orasi Delta Lake integration

set -e

echo "🚀 Setting up Delta Lake Testing Platform for Orasi..."

# Check if Docker and Docker Compose are available
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi

# Create necessary directories
echo "📁 Creating directories..."
mkdir -p data
mkdir -p notebooks
mkdir -p config

# Start the services
echo "🐳 Starting Docker services..."
docker-compose up -d

# Wait for services to be ready
echo "⏳ Waiting for services to be ready..."
sleep 30

# Check service health
echo "🔍 Checking service health..."
if ! docker-compose ps | grep -q "Up"; then
    echo "❌ Some services failed to start. Check logs with: docker-compose logs"
    exit 1
fi

# Initialize MinIO buckets
echo "🪣 Initializing MinIO buckets..."
docker exec orasi-minio mc alias set myminio http://localhost:9000 minioadmin minioadmin || true
docker exec orasi-minio mc mb myminio/test-bucket || true
docker exec orasi-minio mc mb myminio/spark-logs || true
docker exec orasi-minio mc mb myminio/telemetry-data || true

# Test MinIO connectivity
echo "🧪 Testing MinIO connectivity..."
if curl -s http://localhost:9000/minio/health/live > /dev/null; then
    echo "✅ MinIO is running and accessible"
else
    echo "❌ MinIO is not accessible"
fi

# Test Spark connectivity
echo "🧪 Testing Spark connectivity..."
if curl -s http://localhost:8080 > /dev/null; then
    echo "✅ Spark Master is running and accessible"
else
    echo "❌ Spark Master is not accessible"
fi

# Display service information
echo ""
echo "🎉 Delta Lake Testing Platform is ready!"
echo ""
echo "📊 Service URLs:"
echo "  • MinIO Console:     http://localhost:9001 (minioadmin/minioadmin)"
echo "  • Spark Master:      http://localhost:8080"
echo "  • Spark Worker:      http://localhost:8081"
echo "  • Spark History:     http://localhost:18080"
echo "  • Jupyter Notebook:  http://localhost:8888"
echo ""
echo "📁 Directories:"
echo "  • Data:              ./data"
echo "  • Notebooks:         ./notebooks"
echo "  • Configuration:     ./config"
echo ""
echo "🔧 Useful Commands:"
echo "  • View logs:         docker-compose logs -f"
echo "  • Stop services:     docker-compose down"
echo "  • Restart services:  docker-compose restart"
echo "  • Clean up:          docker-compose down -v"
echo ""
echo "📚 Next Steps:"
echo "  1. Open Jupyter Notebook at http://localhost:8888"
echo "  2. Navigate to notebooks/delta_lake_testing.ipynb"
echo "  3. Run the cells to test Delta Lake functionality"
echo "  4. Check the documentation at docs/delta-lake-testing.md"
echo ""
