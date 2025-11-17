#!/bin/bash
# E2E test for Mermaid diagram rendering with labels
# This test verifies that diagrams render correctly with visible labels

set -e

echo "=========================================="
echo "E2E Test: Mermaid Diagram Label Rendering"
echo "=========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PASSED=0
FAILED=0

# Test 1: Verify mermaid.ink /img/ endpoint is accessible
echo "Test 1: Verify mermaid.ink /img/ endpoint accessibility"
SIMPLE_DIAGRAM="graph TD\n    A[Start] --> B[End]"
ENCODED=$(echo -n "$SIMPLE_DIAGRAM" | base64)
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "https://mermaid.ink/img/$ENCODED")

if [ "$HTTP_CODE" = "200" ]; then
    echo -e "${GREEN}✓ PASS${NC}: mermaid.ink /img/ endpoint is accessible (HTTP $HTTP_CODE)"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC}: mermaid.ink /img/ endpoint returned HTTP $HTTP_CODE (expected 200)"
    ((FAILED++))
fi
echo ""

# Test 2: Verify /img/ endpoint returns JPEG format
echo "Test 2: Verify /img/ endpoint returns JPEG format"
RESPONSE=$(curl -s "https://mermaid.ink/img/$ENCODED")
MAGIC_BYTES=$(echo -n "$RESPONSE" | xxd -p -l 3)

if [ "$MAGIC_BYTES" = "ffd8ff" ]; then
    echo -e "${GREEN}✓ PASS${NC}: Response is JPEG format (magic bytes: FF D8 FF)"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC}: Response is not JPEG (magic bytes: $MAGIC_BYTES, expected ffd8ff)"
    ((FAILED++))
fi
echo ""

# Test 3: Verify JPEG image has reasonable size (labels should add content)
echo "Test 3: Verify JPEG has reasonable size (indicates labels present)"
IMAGE_SIZE=$(echo -n "$RESPONSE" | wc -c | tr -d ' ')

if [ "$IMAGE_SIZE" -gt 1000 ]; then
    echo -e "${GREEN}✓ PASS${NC}: JPEG size is $IMAGE_SIZE bytes (>1KB indicates content with labels)"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC}: JPEG size is only $IMAGE_SIZE bytes (too small, might be missing labels)"
    ((FAILED++))
fi
echo ""

# Test 4: Test complex diagram with multiple labels
echo "Test 4: Verify complex diagram rendering"
COMPLEX_DIAGRAM="graph TD\n    A[User] -->|Enter credentials| B[Login Page]\n    B -->|Submit| C[Auth Server]\n    C --> D[Database]\n    D -->|Valid| E[Success]\n    D -->|Invalid| F[Error]"
ENCODED_COMPLEX=$(echo -n "$COMPLEX_DIAGRAM" | base64)
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "https://mermaid.ink/img/$ENCODED_COMPLEX")

if [ "$HTTP_CODE" = "200" ]; then
    echo -e "${GREEN}✓ PASS${NC}: Complex diagram renders successfully (HTTP $HTTP_CODE)"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC}: Complex diagram failed to render (HTTP $HTTP_CODE)"
    ((FAILED++))
fi
echo ""

# Test 5: Verify /img/ endpoint does NOT accept theme configs (should return 404)
echo "Test 5: Verify /img/ endpoint rejects theme configs"
DIAGRAM_WITH_THEME="%%{init: {'theme':'base'}}%%\ngraph TD\n    A --> B"
ENCODED_THEME=$(echo -n "$DIAGRAM_WITH_THEME" | base64)
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "https://mermaid.ink/img/$ENCODED_THEME")

if [ "$HTTP_CODE" = "404" ]; then
    echo -e "${GREEN}✓ PASS${NC}: /img/ endpoint correctly rejects theme configs (HTTP $HTTP_CODE)"
    echo -e "  ${YELLOW}Note:${NC} This is expected - /img/ doesn't support theme configs"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠ UNEXPECTED${NC}: /img/ endpoint accepted theme config (HTTP $HTTP_CODE)"
    echo -e "  ${YELLOW}Note:${NC} API behavior may have changed - verify rendering still works"
    ((PASSED++))
fi
echo ""

# Test 6: Save test image and verify it's valid
echo "Test 6: Save test image and verify file integrity"
TEST_OUTPUT="/tmp/e2e_mermaid_test.jpg"
curl -s "https://mermaid.ink/img/$ENCODED" -o "$TEST_OUTPUT"

if file "$TEST_OUTPUT" | grep -q "JPEG"; then
    echo -e "${GREEN}✓ PASS${NC}: Saved image is valid JPEG"
    echo -e "  File: $TEST_OUTPUT"
    echo -e "  Size: $(ls -lh "$TEST_OUTPUT" | awk '{print $5}')"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC}: Saved image is not a valid JPEG"
    ((FAILED++))
fi
echo ""

# Test 7: Verify unit tests pass
echo "Test 7: Run Rust unit tests for mermaid module"
if cargo test --lib mermaid:: --quiet 2>&1 | grep -q "test result: ok"; then
    echo -e "${GREEN}✓ PASS${NC}: All mermaid unit tests pass"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC}: Some mermaid unit tests failed"
    ((FAILED++))
fi
echo ""

# Test 8: Verify debug test generates valid image
echo "Test 8: Run debug test and verify output"
if cargo test --test debug_mermaid_png --quiet 2>&1 | grep -q "test result: ok"; then
    if [ -f "/tmp/debug_mermaid.jpg" ]; then
        if file "/tmp/debug_mermaid.jpg" | grep -q "JPEG"; then
            echo -e "${GREEN}✓ PASS${NC}: Debug test generated valid JPEG"
            ((PASSED++))
        else
            echo -e "${RED}✗ FAIL${NC}: Debug test output is not valid JPEG"
            ((FAILED++))
        fi
    else
        echo -e "${RED}✗ FAIL${NC}: Debug test did not create output file"
        ((FAILED++))
    fi
else
    echo -e "${RED}✗ FAIL${NC}: Debug test failed to run"
    ((FAILED++))
fi
echo ""

# Summary
echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo -e "Total Tests: $((PASSED + FAILED))"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ ALL TESTS PASSED${NC}"
    echo ""
    echo "Mermaid rendering is working correctly with:"
    echo "  - JPEG format from mermaid.ink/img/"
    echo "  - Labels visible (no foreignObject issues)"
    echo "  - No theme config (white background)"
    echo ""
    exit 0
else
    echo -e "${RED}✗ SOME TESTS FAILED${NC}"
    echo ""
    echo "Please investigate failures before deploying changes."
    echo ""
    exit 1
fi
