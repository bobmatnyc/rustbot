# Rustbot Documentation

Welcome to the Rustbot documentation system. This directory contains comprehensive documentation for the Rustbot AI assistant project.

---

## Quick Navigation

### 🚀 New to Rustbot?

1. **[Project README](../README.md)** - Project overview and quick start
2. **[Quick Start Guide](guides/QUICK_START.md)** - Get Rustbot running in 5 minutes
3. **[Development Guide](../DEVELOPMENT.md)** - Comprehensive development workflow (600+ lines)
4. **[Agent Configuration](../agents/README.md)** - Configure AI agents

### 📚 Documentation Sections

| Section | Description | Key Documents |
|---------|-------------|---------------|
| **[Architecture](architecture/)** | System design, refactoring plans, diagrams | [Best Practices](architecture/best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md), [Refactoring Plan](architecture/planning/RUSTBOT_REFACTORING_PLAN.md), [Diagrams](architecture/diagrams/) |
| **[Guides](guides/)** | Quick start guides and tutorials | [Quick Start](guides/QUICK_START.md), [Refactoring Guide](guides/QUICK_START_REFACTORING.md), [MCP Guide](guides/MCP_QUICKSTART.md) |
| **[QA](qa/)** | Testing, validation, quality assurance | [Testing Methods](qa/TESTING_METHODS.md), [QA Checklist](qa/QA_CHECKLIST.md), [Validation Reports](qa/QA_VALIDATION_REPORT.md) |
| **[Reviews](reviews/)** | Documentation and code reviews | [Documentation Review](reviews/DOCUMENTATION_REVIEW.md) |
| **[Progress](progress/)** | Development session logs | [Latest Sessions](progress/) |
| **[Fixes](fixes/)** | Bug fixes and issue resolutions | [Recent Fixes](fixes/) |

---

## Quick Access by Role

### 👨‍💻 For Developers

**Getting Started:**
1. [Quick Start Guide](guides/QUICK_START.md) - Setup and basic usage
2. [Development Guide](../DEVELOPMENT.md) - Full development workflow
3. [Architecture Overview](architecture/diagrams/ARCHITECTURE_DIAGRAMS.md) - Visual system overview

**Working on Refactoring:**
1. [Refactoring Plan](architecture/planning/RUSTBOT_REFACTORING_PLAN.md) - Overall strategy
2. [Refactoring Checklist](architecture/planning/REFACTORING_CHECKLIST.md) - Track progress
3. [Refactoring Guide](guides/QUICK_START_REFACTORING.md) - Step-by-step workflow

**Implementing Features:**
1. [Architecture Best Practices](architecture/best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md) - Rust patterns
2. [Data Flow](architecture/diagrams/DATA_FLOW.md) - Understand system interactions
3. [Testing Methods](qa/TESTING_METHODS.md) - Test your changes

### 🎯 For QA Engineers

**Testing Workflow:**
1. [Testing Methods](qa/TESTING_METHODS.md) - Comprehensive testing guide
2. [QA Checklist](qa/QA_CHECKLIST.md) - Standard validation procedures
3. [Verification Checklist](qa/VERIFICATION_CHECKLIST.md) - Feature-specific verification

**Validation:**
1. [QA Validation Report](qa/QA_VALIDATION_REPORT.md) - Report template
2. [Prototype Test Results](architecture/implementation/PROTOTYPE_TEST_RESULTS.md) - Example test results

### 🏗️ For Architects

**Architecture Documentation:**
1. [Rust Architecture Best Practices](architecture/best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md) - Patterns and principles
2. [Architecture Research](architecture/planning/ARCHITECTURE_RESEARCH_SUMMARY.md) - Research findings
3. [Architecture Diagrams](architecture/diagrams/ARCHITECTURE_DIAGRAMS.md) - Visual documentation
4. [Data Flow](architecture/diagrams/DATA_FLOW.md) - System interactions

**Planning:**
1. [Refactoring Plan](architecture/planning/RUSTBOT_REFACTORING_PLAN.md) - Strategic direction
2. [Refactoring Timeline](architecture/diagrams/REFACTORING_TIMELINE.md) - Visual timeline

### 📋 For Project Managers

**Status & Progress:**
1. [Progress Logs](progress/) - Recent development sessions
2. [Refactoring Checklist](architecture/planning/REFACTORING_CHECKLIST.md) - Track completion

**Planning:**
1. [Refactoring Plan](architecture/planning/RUSTBOT_REFACTORING_PLAN.md) - Overall strategy
2. [QA Validation Reports](qa/QA_VALIDATION_REPORT.md) - Quality status

### 📖 For New Contributors

**Onboarding Path:**
1. [Quick Start Guide](guides/QUICK_START.md) - Get running (~5 min)
2. [Development Guide](../DEVELOPMENT.md) - Learn workflow (~30 min)
3. [Architecture Overview](architecture/diagrams/ARCHITECTURE_DIAGRAMS.md) - Understand system (~15 min)
4. [Best Practices](architecture/best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md) - Learn patterns (~20 min)
5. [Recent Progress](progress/) - Current context (~10 min)

---

## Quick Access by Task

| Task | Documentation |
|------|---------------|
| **Get Rustbot running** | [Quick Start Guide](guides/QUICK_START.md) |
| **Set up development environment** | [Development Guide](../DEVELOPMENT.md) |
| **Understand system architecture** | [Architecture Diagrams](architecture/diagrams/ARCHITECTURE_DIAGRAMS.md) |
| **Work on refactoring** | [Refactoring Guide](guides/QUICK_START_REFACTORING.md) |
| **Follow best practices** | [Rust Best Practices](architecture/best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md) |
| **Test features** | [Testing Methods](qa/TESTING_METHODS.md) |
| **Validate releases** | [QA Checklist](qa/QA_CHECKLIST.md) |
| **Set up MCP plugins** | [MCP Quick Start](guides/MCP_QUICKSTART.md) |
| **Create diagrams** | [Diagram Creation Guide](architecture/diagrams/DIAGRAM_CREATION_SUMMARY.md) |
| **Review documentation** | [Documentation Review](reviews/DOCUMENTATION_REVIEW.md) |
| **Track progress** | [Progress Logs](progress/) |
| **Fix bugs** | [Bug Fixes](fixes/) |

---

## Documentation Structure

```
docs/
├── README.md (this file)           # Documentation hub and navigation
│
├── architecture/                    # Architecture & design documentation
│   ├── README.md                   # Architecture navigation
│   ├── best-practices/             # Rust architecture best practices
│   ├── planning/                   # Refactoring plans and research
│   ├── implementation/             # Implementation summaries and prototypes
│   └── diagrams/                   # Visual architecture diagrams
│
├── guides/                         # User guides and tutorials
│   ├── README.md                   # Guides navigation
│   ├── QUICK_START.md              # General quick start
│   ├── QUICK_START_REFACTORING.md  # Refactoring workflow guide
│   └── MCP_QUICKSTART.md           # MCP integration guide
│
├── qa/                             # Quality assurance documentation
│   ├── README.md                   # QA navigation
│   ├── TESTING_METHODS.md          # Testing methodologies
│   ├── QA_CHECKLIST.md             # Standard QA checklist
│   ├── VERIFICATION_CHECKLIST.md   # Feature verification
│   └── QA_VALIDATION_REPORT.md     # Validation reports
│
├── reviews/                        # Documentation and code reviews
│   ├── README.md                   # Reviews navigation
│   └── DOCUMENTATION_REVIEW.md     # Documentation review
│
├── progress/                       # Development session logs
│   └── YYYY-MM-DD-*.md             # Dated session logs
│
└── fixes/                          # Bug fixes and issue resolutions
    └── YYYY-MM-DD-*.md             # Dated fix documentation
```

---

## Documentation Standards

### File Naming Conventions

**Documentation Files:**
- Use SCREAMING_SNAKE_CASE for major docs: `ARCHITECTURE.md`, `API.md`
- Use descriptive names: `RUST_ARCHITECTURE_BEST_PRACTICES.md` not `rust.md`
- Topic prefixes for related docs: `REFACTORING_CHECKLIST.md`, `REFACTORING_PLAN.md`

**Progress Logs:**
- Format: `YYYY-MM-DD-topic.md`
- Example: `2025-01-17-architecture-refactoring-session.md`
- Be specific with topic names

**Fix Documentation:**
- Format: `YYYY-MM-DD-issue-description.md`
- Example: `2025-01-14-marketplace-dedup-fix.md`

### Content Standards

Every document should include:

```markdown
---
title: Document Title
category: Architecture | Guide | QA | Review
audience: Developer | PM | QA | All
reading_time: X minutes
last_updated: YYYY-MM-DD
status: Complete | Draft | Deprecated
---

# Document Title

## Overview
Brief description of purpose and scope.

## Content
Well-organized, scannable content with clear sections.

## Examples
Code samples where relevant with syntax highlighting.
```

### Writing Style

- **Clear and concise** - Avoid unnecessary verbosity
- **Active voice** - "The system processes requests" not "Requests are processed"
- **Present tense** - For current state ("The API supports...")
- **Past tense** - For completed work ("We implemented...")
- **Code blocks** - Always specify language for syntax highlighting

---

## Documentation Maintenance

### Regular Maintenance

**Monthly Tasks:**
- Review recent documentation for accuracy
- Update cross-references and links
- Archive old progress logs (>1 month) if needed
- Update this README if structure changed

**Quarterly Reviews:**
- Comprehensive documentation review
- Update architecture docs to reflect current state
- Consolidate or archive redundant content
- Review and update best practices

### Adding New Documentation

**Checklist:**
- [ ] File named clearly and descriptively
- [ ] Metadata header added (category, audience, etc.)
- [ ] Content follows documentation standards
- [ ] Placed in appropriate directory
- [ ] Directory README updated
- [ ] Cross-references updated
- [ ] This README updated if major addition

### Updating Existing Documentation

**Checklist:**
- [ ] Update `last_updated` metadata
- [ ] Verify all links still work
- [ ] Update related cross-references
- [ ] Review for accuracy and completeness
- [ ] Update examples if code changed

---

## Contributing Documentation

### Creating New Documentation

1. **Check existing documentation** - Avoid duplication
2. **Choose appropriate location:**
   - Architecture/design → `architecture/`
   - User guides → `guides/`
   - Testing/QA → `qa/`
   - Reviews → `reviews/`
   - Progress logs → `progress/`
   - Bug fixes → `fixes/`
3. **Follow naming conventions** (see above)
4. **Add metadata header** (see template above)
5. **Update directory README** (if adding to subdirectory)
6. **Update this README** (if major addition)

### Documentation Templates

**See [MAINTENANCE.md](MAINTENANCE.md)** for comprehensive templates including:
- Architecture document template
- Guide template
- QA document template
- Progress log template
- Review template

---

## Related Resources

### Project Documentation
- **[Project README](../README.md)** - Project overview
- **[Development Guide](../DEVELOPMENT.md)** - Comprehensive development workflow
- **[Agent Configuration](../agents/README.md)** - AI agent setup
- **[Version Management](../VERSION_MANAGEMENT.md)** - Release procedures

### External Resources
- **[Rust Book](https://doc.rust-lang.org/book/)** - Learn Rust
- **[egui Documentation](https://docs.rs/egui/)** - UI framework
- **[OpenRouter API](https://openrouter.ai/docs)** - LLM provider

---

## Getting Help

### Documentation Issues

- **Broken link?** Check the file was moved, update reference
- **Outdated content?** Update the source document
- **Can't find something?** Check directory READMEs for navigation
- **Missing documentation?** Create it following standards above

### Development Questions

- **Setup & workflow:** [Development Guide](../DEVELOPMENT.md)
- **Architecture & design:** [Architecture](architecture/)
- **Testing & QA:** [QA Documentation](qa/)
- **Recent changes:** [Progress Logs](progress/)
- **Bug fixes:** [Fixes](fixes/)

---

## Documentation Metadata

**Current Version:** v3.0
**Last Major Update:** 2025-01-17 (Architecture refactoring organization)
**Maintained By:** Documentation Agent and contributors
**Next Review:** 2025-02-17

### Version History

- **v3.0 (2025-01-17)** - Reorganized architecture documentation into logical subdirectories
- **v2.0 (2025-11-15)** - Created INDEX.md catalog, separated active from archived
- **v1.0 (pre-2025-11-15)** - Initial flat structure

---

**Quick Tip:** Each subdirectory has its own README.md with detailed navigation. Start there for section-specific guidance!
