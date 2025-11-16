#!/bin/bash
# Debug script to investigate why web_search tool isn't being called

set -e

echo "🔍 Testing Tool Calling with Enhanced Logging"
echo "=============================================="
echo ""

# Run rustbot with info-level logging to capture tool information
echo "📝 Starting rustbot with enhanced logging..."
echo "   (Will capture logs to /tmp/rustbot_tool_debug.log)"
echo ""

RUST_LOG=info ./target/debug/rustbot > /tmp/rustbot_tool_debug.log 2>&1 &
RUSTBOT_PID=$!

# Wait for startup
sleep 3

# Check if running
if ! ps -p $RUSTBOT_PID > /dev/null 2>&1; then
    echo "❌ Rustbot failed to start"
    cat /tmp/rustbot_tool_debug.log
    exit 1
fi

echo "✅ Rustbot is running (PID: $RUSTBOT_PID)"
echo ""
echo "📋 Instructions for Manual Testing:"
echo "   1. The rustbot GUI is now running"
echo "   2. Ask: 'What's the latest news about AI?'"
echo "   3. After the response, close the app"
echo "   4. This script will analyze the logs"
echo ""
echo "Press Enter when you've tested and closed the app..."
read

# Wait for process to end
wait $RUSTBOT_PID 2>/dev/null || true

echo ""
echo "📊 Analyzing logs for tool calling behavior..."
echo ""

# Check for tool registry
if grep -q "Tool registry updated" /tmp/rustbot_tool_debug.log; then
    echo "✅ Tool Registry:"
    grep "Tool registry" /tmp/rustbot_tool_debug.log
else
    echo "❌ No tool registry update found"
fi

echo ""

# Check for tools being sent to API
if grep -q "Sending.*tools to API" /tmp/rustbot_tool_debug.log; then
    echo "✅ Tools Sent to LLM:"
    grep -A 3 "Sending.*tools to API" /tmp/rustbot_tool_debug.log
else
    echo "⚠️  No evidence of tools being sent to API"
fi

echo ""

# Check for tool_choice
if grep -q "tool_choice" /tmp/rustbot_tool_debug.log; then
    echo "✅ Tool Choice Configuration:"
    grep "tool_choice" /tmp/rustbot_tool_debug.log
else
    echo "⚠️  No tool_choice information found"
fi

echo ""

# Check for tool calls in response
if grep -q "Response contains.*tool call" /tmp/rustbot_tool_debug.log; then
    echo "✅ LLM Made Tool Call(s):"
    grep "Response contains.*tool call" /tmp/rustbot_tool_debug.log
    grep -A 1 "Tool call:" /tmp/rustbot_tool_debug.log || true
else
    if grep -q "Response contains NO tool calls" /tmp/rustbot_tool_debug.log; then
        echo "❌ LLM DID NOT Make Any Tool Calls (responded directly)"
        echo ""
        echo "🔍 This indicates the LLM chose not to use the tool."
        echo "   Possible reasons:"
        echo "   1. Tool description not clear enough"
        echo "   2. Model instructions override tool usage"
        echo "   3. tool_choice not set to 'auto' or 'required'"
    else
        echo "⚠️  No tool call information in logs"
    fi
fi

echo ""
echo "📄 Full debug log saved to: /tmp/rustbot_tool_debug.log"
echo "   Use: cat /tmp/rustbot_tool_debug.log | less"
echo ""
