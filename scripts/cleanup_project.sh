#!/bin/bash

# Rustbot Project Cleanup Script
# Organizes test files, debug docs, and temporary files

set -e  # Exit on error

PROJECT_ROOT="/Users/masa/Projects/rustbot"
cd "$PROJECT_ROOT"

echo "🧹 Starting Rustbot Project Cleanup..."
echo ""

# 1. Create archive structure
echo "📁 Creating archive directories..."
mkdir -p docs/archive/debug
mkdir -p docs/archive/progress/2025-11-12
mkdir -p docs/archive/progress/2025-11-13

# 2. Move debug docs from root to archive
echo "📦 Archiving debug documentation..."
if [ -f "DEBUGGING_EMPTY_MESSAGES.md" ]; then
    mv DEBUGGING_EMPTY_MESSAGES.md docs/archive/debug/
    echo "   ✓ DEBUGGING_EMPTY_MESSAGES.md"
fi

if [ -f "DEBUGGING_TOOL_STATE.md" ]; then
    mv DEBUGGING_TOOL_STATE.md docs/archive/debug/
    echo "   ✓ DEBUGGING_TOOL_STATE.md"
fi

if [ -f "TEST_TOOL_CALLING.md" ]; then
    mv TEST_TOOL_CALLING.md docs/archive/debug/
    echo "   ✓ TEST_TOOL_CALLING.md"
fi

if [ -f "TESTING_INSTRUCTIONS.md" ]; then
    mv TESTING_INSTRUCTIONS.md docs/archive/debug/
    echo "   ✓ TESTING_INSTRUCTIONS.md"
fi

# 3. Archive 2025-11-12 progress logs
echo ""
echo "📦 Archiving 2025-11-12 progress logs..."
mv docs/progress/2025-11-12-*.md docs/archive/progress/2025-11-12/ 2>/dev/null || echo "   (No 2025-11-12 files to move)"

# 4. Archive 2025-11-13 intermediate files
echo ""
echo "📦 Archiving 2025-11-13 intermediate files..."

# Tool calling intermediates (keep final only)
for file in docs/progress/2025-11-13-tool-calling-*.md; do
    if [ -f "$file" ]; then
        mv "$file" docs/archive/progress/2025-11-13/
        echo "   ✓ $(basename $file)"
    fi
done

if [ -f "docs/progress/2025-11-13-tool-call-detection.md" ]; then
    mv docs/progress/2025-11-13-tool-call-detection.md docs/archive/progress/2025-11-13/
    echo "   ✓ 2025-11-13-tool-call-detection.md"
fi

if [ -f "docs/progress/2025-11-13-tool-execution-fix.md" ]; then
    mv docs/progress/2025-11-13-tool-execution-fix.md docs/archive/progress/2025-11-13/
    echo "   ✓ 2025-11-13-tool-execution-fix.md"
fi

if [ -f "docs/progress/2025-11-13-tool-execution-implementation.md" ]; then
    mv docs/progress/2025-11-13-tool-execution-implementation.md docs/archive/progress/2025-11-13/
    echo "   ✓ 2025-11-13-tool-execution-implementation.md"
fi

# Empty message intermediates (keep ROOT-CAUSE-FIX only)
for file in docs/progress/2025-11-13-empty-*.md; do
    if [ -f "$file" ] && [[ "$file" != *"ROOT-CAUSE-FIX"* ]]; then
        mv "$file" docs/archive/progress/2025-11-13/
        echo "   ✓ $(basename $file)"
    fi
done

# Other intermediates
if [ -f "docs/progress/2025-11-13-serialization-debug-logging.md" ]; then
    mv docs/progress/2025-11-13-serialization-debug-logging.md docs/archive/progress/2025-11-13/
    echo "   ✓ 2025-11-13-serialization-debug-logging.md"
fi

if [ -f "docs/progress/2025-11-13-conversation-history-fix.md" ]; then
    mv docs/progress/2025-11-13-conversation-history-fix.md docs/archive/progress/2025-11-13/
    echo "   ✓ 2025-11-13-conversation-history-fix.md"
fi

if [ -f "docs/progress/2025-11-13-debug-logging-and-clear-fix.md" ]; then
    mv docs/progress/2025-11-13-debug-logging-and-clear-fix.md docs/archive/progress/2025-11-13/
    echo "   ✓ 2025-11-13-debug-logging-and-clear-fix.md"
fi

if [ -f "docs/progress/2025-11-13-async-thread-safety-fix.md" ]; then
    mv docs/progress/2025-11-13-async-thread-safety-fix.md docs/archive/progress/2025-11-13/
    echo "   ✓ 2025-11-13-async-thread-safety-fix.md"
fi

if [ -f "docs/progress/2025-11-13-event-bus-performance-analysis.md" ]; then
    mv docs/progress/2025-11-13-event-bus-performance-analysis.md docs/archive/progress/2025-11-13/
    echo "   ✓ 2025-11-13-event-bus-performance-analysis.md"
fi

if [ -f "docs/progress/2025-11-13-agent-delegation-session.md" ]; then
    mv docs/progress/2025-11-13-agent-delegation-session.md docs/archive/progress/2025-11-13/
    echo "   ✓ 2025-11-13-agent-delegation-session.md"
fi

if [ -f "docs/progress/2025-11-13-json-agents-session.md" ]; then
    mv docs/progress/2025-11-13-json-agents-session.md docs/archive/progress/2025-11-13/
    echo "   ✓ 2025-11-13-json-agents-session.md"
fi

# 5. Clean /tmp/ files
echo ""
echo "🗑️  Cleaning old /tmp/ files..."
rm -f /tmp/rustbot_debug.log && echo "   ✓ rustbot_debug.log"
rm -f /tmp/rustbot_test_output.log && echo "   ✓ rustbot_test_output.log"
rm -f /tmp/rustbot_test.log && echo "   ✓ rustbot_test.log"
rm -f /tmp/rustbot_tool_test_output.log && echo "   ✓ rustbot_tool_test_output.log"
rm -f /tmp/rustbot_trace.log && echo "   ✓ rustbot_trace.log"
rm -f /tmp/rustbot.log && echo "   ✓ rustbot.log"
rm -f /tmp/test_gui_api.rs && echo "   ✓ test_gui_api.rs"

# 6. Summary
echo ""
echo "✅ Cleanup complete!"
echo ""
echo "📊 Summary:"
echo "   Root directory: Clean (debug docs archived)"
echo "   Progress logs: Organized (kept final versions)"
echo "   Archived to: docs/archive/"
echo "   /tmp/: Old logs removed"
echo ""
echo "📁 Kept files:"
ls -1 docs/progress/ | wc -l | xargs echo "   docs/progress/: " files
ls -1 docs/archive/debug/ 2>/dev/null | wc -l | xargs echo "   docs/archive/debug/: " files
ls -1 docs/archive/progress/2025-11-12/ 2>/dev/null | wc -l | xargs echo "   docs/archive/progress/2025-11-12/: " files
ls -1 docs/archive/progress/2025-11-13/ 2>/dev/null | wc -l | xargs echo "   docs/archive/progress/2025-11-13/: " files
echo ""
