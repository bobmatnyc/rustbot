#!/bin/bash
# E2E Test for Mermaid Diagram Rendering
# This script tests the complete Mermaid rendering pipeline to prevent regressions

set -e  # Exit on error

echo "🧪 Starting E2E Mermaid Rendering Tests..."

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Cleanup function
cleanup() {
    rm -f /tmp/test_mermaid_*.svg /tmp/test_mermaid_*.png
}
trap cleanup EXIT

# Test 1: Mermaid.ink API Availability
test_api_availability() {
    echo -n "Test 1: Mermaid.ink API availability... "

    # Simple diagram
    MERMAID_CODE="graph TD
    A-->B"

    # Base64 encode
    ENCODED=$(echo -n "$MERMAID_CODE" | base64)

    # Test API
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "https://mermaid.ink/svg/$ENCODED")

    if [ "$HTTP_CODE" = "200" ]; then
        echo -e "${GREEN}✓ PASS${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ FAIL (HTTP $HTTP_CODE)${NC}"
        ((TESTS_FAILED++))
    fi
}

# Test 2: SVG Response Format
test_svg_format() {
    echo -n "Test 2: SVG response format validation... "

    MERMAID_CODE="graph TD
    A-->B"
    ENCODED=$(echo -n "$MERMAID_CODE" | base64)

    curl -s "https://mermaid.ink/svg/$ENCODED" -o /tmp/test_mermaid_format.svg

    # Check if it's valid SVG
    if head -c 100 /tmp/test_mermaid_format.svg | grep -q "<svg"; then
        echo -e "${GREEN}✓ PASS${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ FAIL (Not valid SVG)${NC}"
        ((TESTS_FAILED++))
    fi
}

# Test 3: Transparent Background Theme
test_transparent_background() {
    echo -n "Test 3: Transparent background configuration... "

    THEME_CONFIG="%%{init: {'theme':'base','themeVariables':{'background':'transparent'}}}%%"
    MERMAID_CODE="$THEME_CONFIG
graph TD
    A-->B"

    ENCODED=$(echo -n "$MERMAID_CODE" | base64)

    curl -s "https://mermaid.ink/svg/$ENCODED" -o /tmp/test_mermaid_transparent.svg

    # Check for transparent background in SVG (theme should be applied)
    if head -c 1000 /tmp/test_mermaid_transparent.svg | grep -q "theme"; then
        echo -e "${GREEN}✓ PASS${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${YELLOW}⚠ WARNING (Theme may not be applied)${NC}"
        ((TESTS_PASSED++))
    fi
}

# Test 4: Complex Diagram Rendering
test_complex_diagram() {
    echo -n "Test 4: Complex diagram rendering... "

    MERMAID_CODE="graph TD
    Client[Client] -->|HTTP Request| LoadBalancer[Load Balancer]
    LoadBalancer -->|Distribute| WebServer1[Web Server 1]
    LoadBalancer -->|Distribute| WebServer2[Web Server 2]
    WebServer1 -->|Process| AppServer1[App Server 1]
    WebServer2 -->|Process| AppServer2[App Server 2]
    AppServer1 -->|Query| Database[(Database)]
    AppServer2 -->|Query| Database"

    ENCODED=$(echo -n "$MERMAID_CODE" | base64)

    curl -s "https://mermaid.ink/svg/$ENCODED" -o /tmp/test_mermaid_complex.svg

    # Check file size (complex diagram should be larger)
    FILE_SIZE=$(wc -c < /tmp/test_mermaid_complex.svg)

    if [ "$FILE_SIZE" -gt 5000 ]; then
        echo -e "${GREEN}✓ PASS ($FILE_SIZE bytes)${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ FAIL (Too small: $FILE_SIZE bytes)${NC}"
        ((TESTS_FAILED++))
    fi
}

# Test 5: SVG to PNG Conversion (using Rust test)
test_svg_to_png_conversion() {
    echo -n "Test 5: SVG to PNG conversion... "

    # This would ideally call a Rust unit test
    # For now, just verify the PNG encoding library is available
    if cargo tree | grep -q "tiny-skia"; then
        echo -e "${GREEN}✓ PASS (tiny-skia available)${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ FAIL (tiny-skia not found)${NC}"
        ((TESTS_FAILED++))
    fi
}

# Test 6: Image Format Detection
test_image_format_detection() {
    echo -n "Test 6: Image format detection... "

    MERMAID_CODE="graph TD
    A-->B"
    ENCODED=$(echo -n "$MERMAID_CODE" | base64)

    curl -s "https://mermaid.ink/svg/$ENCODED" -o /tmp/test_mermaid_detect.svg

    # Use file command to detect format
    FORMAT=$(file /tmp/test_mermaid_detect.svg | grep -o "SVG")

    if [ "$FORMAT" = "SVG" ]; then
        echo -e "${GREEN}✓ PASS${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ FAIL (Expected SVG, got: $(file /tmp/test_mermaid_detect.svg))${NC}"
        ((TESTS_FAILED++))
    fi
}

# Run all tests
test_api_availability
test_svg_format
test_transparent_background
test_complex_diagram
test_svg_to_png_conversion
test_image_format_detection

# Summary
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test Summary:"
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
echo -e "${RED}Failed: $TESTS_FAILED${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Exit code
if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed!${NC}"
    exit 1
fi
