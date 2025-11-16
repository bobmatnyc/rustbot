# Rustbot Documentation Index

**Last Updated**: 2025-11-15

Complete catalog of all Rustbot documentation organized by topic and purpose.

---

## Quick Navigation

**Getting Started:**
- [Project Overview](#project-overview) - Start here
- [Development Setup](#development--workflow) - Developer guide
- [Agent Configuration](#agents--configuration) - Configure AI agents

**Architecture & Design:**
- [System Architecture](#architecture--design) - How Rustbot works
- [API Reference](#api--integration) - Programmatic access

**Development:**
- [Progress Logs](#progress-logs) - Recent development history
- [Historical Archive](#historical-archive) - Old docs and fixes

---

## Project Overview

### README.md (Root)
**Location**: `/README.md`
**Purpose**: Main project overview and quick start guide
**Audience**: New users, contributors, anyone discovering the project
**Contains**:
- Project description and current status
- Quick start instructions
- Phase 1 goals and progress
- Technology stack
- Basic project structure

**When to read**: First time encountering Rustbot, need quick overview

### CLAUDE.md (Root)
**Location**: `/CLAUDE.md`
**Purpose**: AI assistant integration guide and project memory configuration
**Audience**: AI assistants (Claude, etc.), developers using AI tools
**Contains**:
- KuzuMemory integration setup
- Project context and technologies
- Development guidelines for AI
- Session progress tracking requirements
- Memory management instructions

**When to read**: Setting up AI assistant integration, understanding project conventions

---

## Development & Workflow

### DEVELOPMENT.md (Root)
**Location**: `/DEVELOPMENT.md`
**Purpose**: Comprehensive 600+ line development workflow guide
**Audience**: Active developers, contributors
**Contains**:
- **Quick Reference**: What requires rebuild vs restart
- **Development Workflows**: Automated (cargo-watch) and manual
- **Agent Configuration**: JSON validation and iteration
- **Environment Variables**: Setup and management
- **Troubleshooting**: Common issues and solutions
- **Advanced Topics**: Debugging, performance, multi-terminal setup
- **Best Practices**: Code, config, and version management
- **Workflow Examples**: Real-world development scenarios

**When to read**:
- Daily development (reference guide)
- Troubleshooting build/config issues
- Learning efficient development workflows
- Setting up cargo-watch automation

**Complements**: README.md (quick start) - this is the complete reference

### VERSION_MANAGEMENT.md (Root)
**Location**: `/VERSION_MANAGEMENT.md`
**Purpose**: Version numbering and release management guide
**Audience**: Developers preparing releases, version bumps
**Contains**:
- Version format explanation (MAJOR.MINOR.PATCH-BUILD)
- Current version tracking
- Update procedures for each version type
- Semantic versioning rules
- Example workflows for releases
- Files to update checklist

**When to read**:
- Making a release
- Bumping version numbers
- Understanding versioning scheme

---

## Architecture & Design

### Core Architecture

#### ARCHITECTURE.md
**Location**: `/docs/ARCHITECTURE.md`
**Purpose**: Core event-driven system architecture specification
**Audience**: Developers, architects, anyone understanding system design
**Contains**:
- Executive summary of architecture
- System layers (UI, Core, Services)
- Design principles
- Event-driven architecture overview
- Component descriptions

**When to read**: Understanding overall system design, architectural decisions

#### AGENT_EVENT_ARCHITECTURE.md
**Location**: `/docs/AGENT_EVENT_ARCHITECTURE.md`
**Purpose**: Comprehensive 46KB agent and event system architecture
**Audience**: Developers working on agent system, event bus, multi-agent features
**Contains**:
- Research summary on event-driven patterns
- Event system design and implementation
- Agent architecture details
- Event routing and filtering
- Message handling patterns
- Architecture improvements and evolution

**When to read**:
- Implementing agent features
- Working on event system
- Understanding multi-agent architecture
- Need detailed implementation guidance

### Feature Design Documents (docs/design/)

#### AGENT_DELEGATION_DESIGN.md
**Location**: `/docs/design/AGENT_DELEGATION_DESIGN.md`
**Purpose**: Delegation pattern design for agent specialization
**Audience**: Developers implementing agent delegation features
**Contains**:
- Delegation flow and patterns
- Specialist agent design
- Tool calling integration

**When to read**: Implementing or modifying agent delegation

#### CONTEXT_MANAGEMENT_DESIGN.md
**Location**: `/docs/design/CONTEXT_MANAGEMENT_DESIGN.md`
**Purpose**: Context window management and compaction design
**Audience**: Developers working on conversation history, token management
**Contains**:
- Context compaction strategies
- Token tracking
- History management

**When to read**: Working on conversation context, optimizing token usage

#### EVENT_VISUALIZATION_DESIGN.md
**Location**: `/docs/design/EVENT_VISUALIZATION_DESIGN.md`
**Purpose**: Event stream visualization feature design
**Audience**: Developers implementing UI visualization features
**Contains**:
- Visualization requirements
- UI design specifications
- Event display patterns

**When to read**: Implementing event visualization UI

#### TOOL_REGISTRATION_DESIGN.md
**Location**: `/docs/design/TOOL_REGISTRATION_DESIGN.md`
**Purpose**: Tool calling system design and registration
**Audience**: Developers working on tool calling, agent capabilities
**Contains**:
- Tool definition format
- Registration flow
- Tool call routing
- Implementation components

**When to read**: Implementing or extending tool calling features

#### PROTOCOL_RESEARCH_FINDINGS.md
**Location**: `/docs/design/PROTOCOL_RESEARCH_FINDINGS.md`
**Purpose**: Research findings for protocol design decisions
**Audience**: Developers making protocol/architecture decisions
**Contains**:
- Protocol research
- Design alternatives
- Decision rationale

**When to read**: Understanding why certain protocols/patterns were chosen

---

## API & Integration

### API.md
**Location**: `/docs/API.md`
**Purpose**: Complete programmatic API documentation
**Audience**: Developers integrating Rustbot, writing scripts, testing
**Contains**:
- Quick start with code examples
- RustbotApi core API reference
- Message operations (blocking & streaming)
- Agent management
- History management
- Event subscription
- Builder pattern usage
- Architecture & implementation details

**When to read**:
- Writing scripts or automation with Rustbot
- Integrating Rustbot into other applications
- Testing without UI
- Understanding programmatic access to all features

---

## Agents & Configuration

### agents/README.md
**Location**: `/agents/README.md`
**Purpose**: Agent configuration system documentation
**Audience**: Users creating custom agents, configuring AI behavior
**Contains**:
- Directory structure explanation
- Quick start for loading agents
- Custom agent creation guide
- Configuration schema reference
- Supported providers
- Example configurations
- Environment variable usage

**When to read**:
- Creating custom agents
- Modifying agent behavior
- Understanding agent JSON schema
- Setting up new LLM providers

---

## Development History

### Development Tracking (docs/development/)

#### REFACTORING.md
**Location**: `/docs/development/REFACTORING.md`
**Purpose**: Code refactoring history and metrics
**Audience**: Developers understanding codebase evolution
**Contains**:
- Module extraction progress
- Line count reduction metrics
- Phase-by-phase improvements
- Benefits achieved
- File structure evolution

**When to read**: Understanding codebase structure changes over time

---

## Progress Logs

### Current Development (docs/progress/)

Recent development session logs documenting current work:

**2025-11-15 Sessions** (Current):
- `2025-11-15-session.md` - Main session overview
- `2025-11-15-tool-calling-debug.md` - Tool calling debugging
- `2025-11-15-tool-calling-diagnosis-complete.md` - Tool calling diagnosis complete
- `2025-11-15-tool-calling-success.md` - Tool calling implementation success
- `2025-11-15-reload-config-feature.md` - Config reload feature
- `2025-11-15-agent-config-verification.md` - Agent config verification
- `2025-11-15-documentation-and-dev-utilities.md` - Documentation cleanup

**2025-11-14 Sessions**:
- `2025-11-14-web-search-verification.md` - Web search feature verification

**2025-11-13 Sessions** (Recent):
- `2025-11-13-tool-execution-complete.md` - Tool execution system complete
- `2025-11-13-empty-content-bug-ROOT-CAUSE-FIX.md` - Empty content bug fixed
- `2025-11-13-performance-optimization-complete.md` - Performance optimizations
- `2025-11-13-web-search-plugins-fix.md` - Web search plugins implementation
- `2025-11-13-panic-fix-env-loading.md` - Environment loading panic fix
- `2025-11-13-runtime-panic-fix.md` - Runtime panic fix
- `2025-11-13-default-model-change.md` - Default model configuration
- `2025-11-13-openrouter-web-search-fix.md` - OpenRouter web search fix

**When to read**:
- Understanding recent development work
- Getting context on current features
- Learning from bug fixes and solutions
- Continuing previous work

---

## Historical Archive

### Archived Documentation (docs/archive/)

#### Bug Fixes (docs/archive/fixes/)
- `TOOL_CALLING_FIX.md` - Historical tool calling bug fix
- `TOOL_EXECUTION_STATUS.md` - Tool execution implementation status
- `fix-empty-content-bug.md` - Empty content bug fix documentation

**When to reference**: Understanding historical bug fixes, similar issues

#### Debug Documentation (docs/archive/debug/)
- `DEBUGGING_EMPTY_MESSAGES.md` - Empty message debugging (FIXED)
- `DEBUGGING_TOOL_STATE.md` - Tool state debugging guide
- `TEST_TOOL_CALLING.md` - Tool calling test guide
- `TESTING_INSTRUCTIONS.md` - Testing instructions

**When to reference**: Debugging similar issues, test methodology

#### Older Progress Logs (docs/archive/progress/)

**2025-11-13**: 16 intermediate development files
**2025-11-12**: 2 early session files

**When to reference**: Historical context, understanding feature evolution

#### Other Archived Docs (docs/archive/)
- `BUGFIXES.md` - Historical bug fix documentation
- `COMPILE_TIME_ICON.md` - Icon optimization reference
- `PERSISTENCE.md` - Persistence implementation verification
- `TESTING.md` - Testing strategy documentation

---

## Product & Planning

### PRD/development-plan.md
**Location**: `/docs/PRD/development-plan.md`
**Purpose**: Project roadmap and phase planning
**Audience**: Project managers, contributors, stakeholders
**Contains**:
- Phase 1 completion status
- Implementation steps
- Success criteria
- Technology decisions
- Future roadmap

**When to read**: Understanding project direction, planning future work

---

## Asset Documentation

### assets/README.md
**Location**: `/assets/README.md`
**Purpose**: Icon and asset documentation
**Audience**: Developers working with UI assets
**Contains**:
- Icon asset information
- Asset processing
- Usage guidelines

**When to read**: Working with UI assets, icons, images

---

## Documentation by Use Case

### "I want to..."

**...get started with Rustbot**
1. Read `/README.md` (overview)
2. Read `/DEVELOPMENT.md` (setup)
3. Read `/agents/README.md` (configure agents)

**...understand the architecture**
1. Read `/docs/ARCHITECTURE.md` (core design)
2. Read `/docs/AGENT_EVENT_ARCHITECTURE.md` (detailed agent system)
3. Review `/docs/design/` for specific features

**...use Rustbot programmatically**
1. Read `/docs/API.md` (complete API reference)
2. Review examples in API.md

**...create a custom agent**
1. Read `/agents/README.md` (agent configuration)
2. Copy example from `agents/presets/`
3. Follow schema in `agents/schema/agent.schema.json`

**...contribute to development**
1. Read `/README.md` (overview)
2. Read `/DEVELOPMENT.md` (comprehensive workflow guide)
3. Read `/CLAUDE.md` (AI integration)
4. Review `/docs/progress/` (recent work)

**...understand recent changes**
1. Review `/docs/progress/2025-11-15-*.md` (current work)
2. Check `/docs/development/REFACTORING.md` (major changes)

**...debug an issue**
1. Check `/DEVELOPMENT.md` Troubleshooting section
2. Review `/docs/progress/` for similar issues
3. Check `/docs/archive/fixes/` for historical fixes

**...make a release**
1. Read `/VERSION_MANAGEMENT.md` (version procedures)
2. Update version files as specified
3. Document in progress log

---

## Documentation Maintenance

### How to Keep This Index Updated

1. **New document created**: Add entry to appropriate section
2. **Document moved**: Update location references
3. **Document deleted**: Remove from index
4. **Monthly review**: Verify all links, update descriptions

### Link Verification

Verify all links remain valid:
```bash
# Check all markdown files exist
find docs -name "*.md" -type f

# Verify references in INDEX.md match actual files
```

---

## Related Files

- **`docs/README.md`**: Documentation overview and navigation guide
- **`docs/CLEANUP_DECISIONS.md`**: Documentation reorganization decisions and rationale
- **This file** (`docs/INDEX.md`): Complete documentation catalog (you are here)

---

**Index maintained by**: Documentation Agent
**Last comprehensive review**: 2025-11-15
**Next review due**: 2025-12-15
