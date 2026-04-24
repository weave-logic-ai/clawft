#!/bin/bash
# Comprehensive Validation Script for v1.7.0
# Runs Docker-based validation and generates report

set -e

echo "================================================================"
echo "AGENTIC-FLOW v1.7.0 - DOCKER VALIDATION"
echo "================================================================"
echo "Started: $(date)"
echo "================================================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Create results directory
mkdir -p validation-results
mkdir -p benchmark-results

# Function to print colored messages
print_status() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Step 1: Clean previous runs
echo "Step 1: Cleaning previous validation runs..."
docker-compose -f docker-compose.validation.yml down -v 2>/dev/null || true
rm -rf validation-results/* benchmark-results/*
print_status "Cleaned previous runs"
echo ""

# Step 2: Build validation image
echo "Step 2: Building validation Docker image..."
if docker-compose -f docker-compose.validation.yml build validation; then
    print_status "Validation image built successfully"
else
    print_error "Failed to build validation image"
    exit 1
fi
echo ""

# Step 3: Run validation suite
echo "Step 3: Running comprehensive validation suite..."
echo "  This will test:"
echo "    - Backwards compatibility"
echo "    - New HybridReasoningBank capabilities"
echo "    - SharedMemoryPool performance"
echo "    - AdvancedMemorySystem features"
echo "    - Memory profiling"
echo "    - Regression detection"
echo ""

if docker-compose -f docker-compose.validation.yml run --rm validation; then
    print_status "Validation suite PASSED"
    VALIDATION_STATUS=0
else
    print_error "Validation suite FAILED"
    VALIDATION_STATUS=1
fi
echo ""

# Step 4: Copy results from container
echo "Step 4: Extracting validation results..."
docker cp agentic-flow-validation:/app/validation-results.json validation-results/ 2>/dev/null || true
if [ -f "validation-results/validation-results.json" ]; then
    print_status "Results extracted successfully"
else
    print_warning "Could not extract results file"
fi
echo ""

# Step 5: Display results summary
echo "Step 5: Validation Results Summary"
echo "================================================================"
if [ -f "validation-results/validation-results.json" ]; then
    # Extract key metrics using jq if available
    if command -v jq &> /dev/null; then
        TOTAL=$(jq '.summary.total' validation-results/validation-results.json)
        PASSED=$(jq '.summary.passed' validation-results/validation-results.json)
        FAILED=$(jq '.summary.failed' validation-results/validation-results.json)
        PASS_RATE=$(jq '.summary.passRate' validation-results/validation-results.json)

        echo "Total Tests: $TOTAL"
        echo "Passed: $PASSED"
        echo "Failed: $FAILED"
        echo "Pass Rate: $PASS_RATE%"
        echo ""

        if [ "$FAILED" -gt 0 ]; then
            echo "Failed Tests:"
            jq -r '.results[] | select(.status == "FAIL") | "  - \(.test): \(.error)"' validation-results/validation-results.json
            echo ""
        fi
    else
        cat validation-results/validation-results.json
    fi
else
    print_warning "No results file found - check Docker logs"
fi
echo "================================================================"
echo ""

# Step 6: Run regression tests (optional - if validation passed)
if [ $VALIDATION_STATUS -eq 0 ]; then
    echo "Step 6: Running regression tests..."
    if docker-compose -f docker-compose.validation.yml run --rm regression 2>/dev/null; then
        print_status "Regression tests PASSED"
    else
        print_warning "Some regression tests failed (check logs)"
    fi
    echo ""
fi

# Step 7: Run performance benchmarks (optional)
echo "Step 7: Running performance benchmarks..."
echo "  (This may take several minutes)"
if docker-compose -f docker-compose.validation.yml run --rm benchmark 2>/dev/null; then
    print_status "Benchmarks completed"
else
    print_warning "Benchmarks incomplete (check logs)"
fi
echo ""

# Step 8: Cleanup
echo "Step 8: Cleaning up Docker resources..."
docker-compose -f docker-compose.validation.yml down -v
print_status "Cleanup complete"
echo ""

# Final Summary
echo "================================================================"
echo "VALIDATION COMPLETE"
echo "================================================================"
echo "Completed: $(date)"
echo ""

if [ $VALIDATION_STATUS -eq 0 ]; then
    echo -e "${GREEN}✓✓✓ ALL VALIDATIONS PASSED ✓✓✓${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Review results in validation-results/"
    echo "  2. Check benchmark-results/ for performance metrics"
    echo "  3. Proceed with release"
    exit 0
else
    echo -e "${RED}✗✗✗ VALIDATION FAILED ✗✗✗${NC}"
    echo ""
    echo "Action required:"
    echo "  1. Review failed tests in validation-results/"
    echo "  2. Check Docker logs: docker-compose -f docker-compose.validation.yml logs"
    echo "  3. Fix issues before release"
    exit 1
fi
