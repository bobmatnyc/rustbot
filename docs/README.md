# Rustbot Documentation

Welcome to the Rustbot documentation system. This directory contains comprehensive documentation for the Rustbot AI assistant project.

---

## Quick Start

### New to Rustbot?

1. **[Project README](../README.md)** - Start here for project overview and quick start
2. **[Development Guide](../DEVELOPMENT.md)** - Complete workflow guide (600+ lines)
3. **[Agent Configuration](../agents/README.md)** - Configure AI agents

### Looking for Something Specific?

**See [INDEX.md](INDEX.md) for the complete documentation catalog** organized by topic with descriptions and use cases.

---

## Documentation Organization

### Root Directory (`/`)
Essential project documentation:
- **README.md** - Project overview and quick start
- **DEVELOPMENT.md** - Comprehensive development workflow guide
- **CLAUDE.md** - AI assistant integration guide
- **VERSION_MANAGEMENT.md** - Version numbering and release guide

### Core Documentation (`docs/`)

```
docs/
├── README.md (this file)      # Documentation overview
├── INDEX.md                    # Complete documentation catalog
├── ARCHITECTURE.md             # Core system architecture
├── AGENT_EVENT_ARCHITECTURE.md # Agent/event system (46KB detailed spec)
├── API.md                      # Complete API reference
│
├── design/                     # Feature design documents
│   ├── AGENT_DELEGATION_DESIGN.md
│   ├── CONTEXT_MANAGEMENT_DESIGN.md
│   ├── EVENT_VISUALIZATION_DESIGN.md
│   ├── TOOL_REGISTRATION_DESIGN.md
│   └── PROTOCOL_RESEARCH_FINDINGS.md
│
├── development/                # Development history
│   └── REFACTORING.md
│
├── PRD/                        # Product & planning
│   └── development-plan.md
│
├── progress/                   # Current development logs
│   ├── 2025-11-15-*.md        # Current work
│   ├── 2025-11-14-*.md
│   └── 2025-11-13-*.md        # Recent work
│
└── archive/                    # Historical documentation
    ├── fixes/                  # Historical bug fixes
    ├── debug/                  # Old debug docs
    └── progress/               # Old progress logs
        ├── 2025-11-13/
        └── 2025-11-12/
```

---

## How to Navigate the Documentation

### By Role

**New Developer:**
1. Read `/README.md` (10 min)
2. Read `/DEVELOPMENT.md` (30 min)
3. Skim `/docs/ARCHITECTURE.md` (15 min)
4. Browse `/docs/progress/` for recent context (10 min)

**Experienced Contributor:**
1. Check `/docs/progress/` for latest work
2. Review `/docs/INDEX.md` for specific topics
3. Reference `/DEVELOPMENT.md` for workflows
4. Consult `/docs/design/` for feature specs

**API User / Integrator:**
1. Read `/docs/API.md` (complete API reference)
2. Reference `/agents/README.md` for agent setup
3. Check examples in API.md

**Project Manager / Stakeholder:**
1. Read `/README.md` (project status)
2. Review `/docs/PRD/development-plan.md` (roadmap)
3. Check `/docs/progress/` for recent updates

### By Topic

**Architecture & Design:**
- Core: `/docs/ARCHITECTURE.md`
- Agents: `/docs/AGENT_EVENT_ARCHITECTURE.md`
- Features: `/docs/design/`

**Development:**
- Workflow: `/DEVELOPMENT.md`
- History: `/docs/development/REFACTORING.md`
- Recent: `/docs/progress/`

**API & Integration:**
- API Reference: `/docs/API.md`
- Agents: `/agents/README.md`

**Planning:**
- Roadmap: `/docs/PRD/development-plan.md`
- Versions: `/VERSION_MANAGEMENT.md`

**See [INDEX.md](INDEX.md) for complete catalog with detailed descriptions.**

---

## Common Questions

### Where do I find...?

**Q: How do I set up my development environment?**
A: Read `/DEVELOPMENT.md` - comprehensive 600+ line guide covering everything from installation to advanced workflows.

**Q: How do I create a custom agent?**
A: Read `/agents/README.md` - complete guide with examples and schema.

**Q: What's the overall architecture?**
A: Start with `/docs/ARCHITECTURE.md`, then dive into `/docs/AGENT_EVENT_ARCHITECTURE.md` for details.

**Q: How do I use Rustbot programmatically?**
A: Read `/docs/API.md` - complete API reference with code examples.

**Q: What changed recently?**
A: Check `/docs/progress/2025-11-15-*.md` for current work.

**Q: How do I troubleshoot build issues?**
A: See `/DEVELOPMENT.md` Troubleshooting section - covers common issues and solutions.

**Q: Where is the changelog?**
A: Check `/docs/progress/` for detailed development logs organized by date.

**Q: How do I make a release?**
A: Follow `/VERSION_MANAGEMENT.md` for version procedures.

### How do I contribute documentation?

1. **Check if doc already exists**: Review [INDEX.md](INDEX.md)
2. **Determine category**:
   - Core architecture → `/docs/ARCHITECTURE.md` or create new
   - Feature design → `/docs/design/`
   - Development process → `/docs/development/`
   - Progress log → `/docs/progress/YYYY-MM-DD-topic.md`
3. **Follow existing patterns**: Match tone and structure of similar docs
4. **Update index**: Add entry to `/docs/INDEX.md`

---

## Documentation Standards

### File Naming

**Active Documentation:**
- Use descriptive names: `AGENT_EVENT_ARCHITECTURE.md` not `agents.md`
- SCREAMING_SNAKE_CASE for major docs: `API.md`, `ARCHITECTURE.md`
- Topic prefixes for related docs: `TOOL_REGISTRATION_DESIGN.md`

**Progress Logs:**
- Format: `YYYY-MM-DD-topic.md`
- Example: `2025-11-15-tool-calling-success.md`
- Be specific: `tool-calling-success` not just `updates`

**Archived Docs:**
- Preserve original names
- Organize in dated subdirectories: `archive/progress/YYYY-MM/`

### Content Standards

**Every document should have:**
1. **Title/Header**: Clear document purpose
2. **Date/Status**: When created, current status
3. **Audience**: Who should read this
4. **Purpose**: What problem does this solve
5. **Content**: Well-organized, scannable
6. **Examples**: Code samples where relevant

**Writing Style:**
- Clear and concise
- Active voice
- Present tense for current state
- Past tense for completed work
- Code blocks with syntax highlighting

### Organization Principles

1. **One authoritative source per topic** - No duplicate content
2. **Logical categorization** - Related docs grouped together
3. **Clear hierarchy** - Core docs vs. feature docs vs. historical
4. **Discoverability** - INDEX.md catalog, descriptive names
5. **Preservation** - Archive historical docs, don't delete

---

## Documentation Maintenance

### Monthly Tasks
1. Archive progress logs older than 1 month to `archive/progress/YYYY-MM/`
2. Review active docs for accuracy
3. Check for redundant content
4. Update INDEX.md with new docs
5. Verify links in INDEX.md and this file

### Quarterly Reviews
1. Review all active documentation for currency
2. Identify and combine redundant content
3. Update architecture docs to reflect current state
4. Archive superseded design docs
5. Update this README if structure changed

### When Adding Documentation

**Checklist:**
- [ ] File named clearly and descriptively
- [ ] Content follows documentation standards
- [ ] Added to appropriate directory
- [ ] Entry added to INDEX.md
- [ ] Cross-references updated if needed
- [ ] README.md updated if major addition

---

## Documentation Versions

**Current Structure**: v2.0 (2025-11-15)
- Reorganized with clear categorization
- Created INDEX.md catalog
- Separated active from archived
- Combined redundant content

**Previous Structure**: v1.0 (pre-2025-11-15)
- Flat structure in docs/
- Some redundancy
- Less organized progress logs

**See [CLEANUP_DECISIONS.md](CLEANUP_DECISIONS.md) for complete reorganization history.**

---

## Quick Reference Links

### Essential Reading
- [Project README](../README.md)
- [Development Guide](../DEVELOPMENT.md) (comprehensive)
- [Complete Documentation Index](INDEX.md)

### Technical References
- [Architecture Specification](ARCHITECTURE.md)
- [Agent/Event System](AGENT_EVENT_ARCHITECTURE.md)
- [API Documentation](API.md)

### Configuration
- [Agent Setup](../agents/README.md)
- [Version Management](../VERSION_MANAGEMENT.md)

### Recent Work
- [Latest Progress Logs](progress/)
- [Refactoring History](development/REFACTORING.md)

### Planning
- [Development Roadmap](PRD/development-plan.md)

---

## Getting Help

### Documentation Issues
- Broken link? Update INDEX.md and this file
- Outdated content? Update the source doc
- Can't find something? Check INDEX.md catalog
- Need clarification? Check DEVELOPMENT.md first

### Development Questions
- **Setup**: See DEVELOPMENT.md
- **Architecture**: See ARCHITECTURE.md
- **API**: See API.md
- **Agents**: See agents/README.md
- **Recent Work**: See docs/progress/

---

## About This Documentation System

**Purpose**: Provide comprehensive, well-organized, and easily navigable documentation for all aspects of Rustbot.

**Principles**:
- **Completeness**: Cover all aspects of the project
- **Organization**: Logical structure, easy to find information
- **Clarity**: Clear, concise, accurate content
- **Currency**: Keep documentation up-to-date
- **Preservation**: Archive historical information, don't delete

**Maintained by**: Documentation Agent and project contributors

**Last major update**: 2025-11-15 (v2.0 reorganization)
**Next review**: 2025-12-15

---

**Quick tip**: If you're not sure where to start, read [INDEX.md](INDEX.md) for the complete catalog with "I want to..." use cases!
