#!/bin/bash
# Analyze the most recent rustbot debug logs

LOG_FILE="/tmp/rustbot_debug.log"

if [ ! -f "$LOG_FILE" ]; then
    echo "❌ No log file found at $LOG_FILE"
    echo "   Run rustbot first to generate logs"
    exit 1
fi

echo "📊 Analyzing Rustbot Logs"
echo "========================="
echo ""

echo "1️⃣  Agent Loading:"
grep "Loaded agent" "$LOG_FILE" || echo "   No agent loading logs found"
echo ""

echo "2️⃣  Tool Registry:"
grep "Tool registry" "$LOG_FILE" || echo "   No tool registry logs found"
echo ""

echo "3️⃣  Tools Sent to API:"
grep -A 5 "Sending.*tools to API" "$LOG_FILE" || echo "   No tool sending logs found"
echo ""

echo "4️⃣  Tool Choice Configuration:"
grep "tool_choice" "$LOG_FILE" || echo "   No tool_choice logs found"
echo ""

echo "5️⃣  LLM Response Tool Calls:"
if grep -q "Response contains.*tool call" "$LOG_FILE"; then
    grep "Response contains" "$LOG_FILE"
    grep -A 1 "Tool call:" "$LOG_FILE" || true
elif grep -q "Response contains NO tool calls" "$LOG_FILE"; then
    echo "   ❌ LLM responded directly (did NOT use tools)"
else
    echo "   No tool call response logs found"
fi
echo ""

echo "6️⃣  Tool Execution:"
grep "Executing tool" "$LOG_FILE" || echo "   No tool execution logs found"
echo ""

echo "📄 Full log available at: $LOG_FILE"
