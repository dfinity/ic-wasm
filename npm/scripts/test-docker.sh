#!/bin/bash

# Test ic-wasm npm package using Docker
# This allows testing without affecting your local npm installation

set -e

# Ensure script is run from npm directory
if [ ! -d "ic-wasm" ] || [ ! -f "ic-wasm/package.json" ]; then
  echo "Error: This script must be run from the npm directory"
  echo "Usage: cd npm && ./scripts/test-docker.sh [quick|full|interactive|clean]"
  exit 1
fi

echo "=========================================="
echo "ic-wasm Docker Testing Suite"
echo "=========================================="
echo ""

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is not installed or not in PATH"
    echo "Please install Docker: https://docs.docker.com/get-docker/"
    exit 1
fi

# Parse arguments
TEST_TYPE=${1:-quick}

case $TEST_TYPE in
    quick)
        echo "Running quick test (Node 20, linux/amd64)..."
        echo ""
        docker build -f Dockerfile.test -t ic-wasm-test:latest .
        docker run --rm ic-wasm-test:latest
        ;;
    
    full)
        echo "Running full test suite (multiple Node versions)..."
        echo ""
        if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null 2>&1; then
            echo "Error: docker-compose is not installed"
            echo "Please install docker-compose: https://docs.docker.com/compose/install/"
            exit 1
        fi
        
        # Use docker compose (new) or docker-compose (old)
        if docker compose version &> /dev/null 2>&1; then
            COMPOSE_CMD="docker compose"
        else
            COMPOSE_CMD="docker-compose"
        fi
        
        echo "=========================================="
        echo "Testing on Node 18..."
        echo "=========================================="
        $COMPOSE_CMD -f docker-compose.test.yml build test-node18
        $COMPOSE_CMD -f docker-compose.test.yml run --rm test-node18
        echo ""
        echo "✓ Node 18 tests passed"
        echo ""
        
        echo "=========================================="
        echo "Testing on Node 20..."
        echo "=========================================="
        $COMPOSE_CMD -f docker-compose.test.yml build test-node20
        $COMPOSE_CMD -f docker-compose.test.yml run --rm test-node20
        echo ""
        echo "✓ Node 20 tests passed"
        echo ""
        
        echo "=========================================="
        echo "Testing on Node 22..."
        echo "=========================================="
        $COMPOSE_CMD -f docker-compose.test.yml build test-node22
        $COMPOSE_CMD -f docker-compose.test.yml run --rm test-node22
        echo ""
        echo "✓ Node 22 tests passed"
        echo ""
        
        echo "=========================================="
        echo "Testing on Node 24..."
        echo "=========================================="
        $COMPOSE_CMD -f docker-compose.test.yml build test-node24
        $COMPOSE_CMD -f docker-compose.test.yml run --rm test-node24
        echo ""
        echo "✓ Node 24 tests passed"
        echo ""
        ;;
    
    interactive)
        echo "Starting interactive testing environment..."
        echo ""
        docker build -f Dockerfile.test -t ic-wasm-test:latest .
        docker run --rm -it ic-wasm-test:latest /bin/bash
        ;;
    
    clean)
        echo "Cleaning up Docker images..."
        docker rmi ic-wasm-test:latest 2>/dev/null || true
        if command -v docker-compose &> /dev/null || docker compose version &> /dev/null 2>&1; then
            if docker compose version &> /dev/null 2>&1; then
                docker compose -f docker-compose.test.yml down --rmi all 2>/dev/null || true
            else
                docker-compose -f docker-compose.test.yml down --rmi all 2>/dev/null || true
            fi
        fi
        echo "Cleanup complete!"
        ;;
    
    *)
        echo "Usage: $0 [quick|full|interactive|clean]"
        echo ""
        echo "Options:"
        echo "  quick       - Quick test on Node 20 (default)"
        echo "  full        - Test on multiple Node versions"
        echo "  interactive - Start an interactive bash shell for manual testing"
        echo "  clean       - Remove all test Docker images"
        echo ""
        echo "Examples:"
        echo "  $0                    # Run quick test"
        echo "  $0 quick              # Run quick test"
        echo "  $0 full               # Run full test suite"
        echo "  $0 interactive        # Interactive testing"
        echo "  $0 clean              # Clean up"
        exit 1
        ;;
esac

echo ""
echo "=========================================="
echo "✓ All tests completed successfully!"
echo "=========================================="
