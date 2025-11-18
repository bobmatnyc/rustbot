# Guides Documentation

## Purpose
This directory contains user guides, quick start documentation, and step-by-step tutorials for working with Rustbot.

## Documents

### Getting Started
- **[QUICK_START.md](QUICK_START.md)** - General quick start guide for Rustbot
  - Reading time: ~5 minutes
  - Audience: All users
  - Covers: Installation, basic usage, configuration

- **[QUICK_START_REFACTORING.md](QUICK_START_REFACTORING.md)** - Quick start guide for refactoring work
  - Reading time: ~10 minutes
  - Audience: Developers working on refactoring
  - Covers: Refactoring workflow, testing, validation

### Feature-Specific Guides
- **[MCP_QUICKSTART.md](MCP_QUICKSTART.md)** - Quick start guide for MCP (Model Context Protocol) integration
  - Reading time: ~8 minutes
  - Audience: Developers integrating MCP plugins
  - Covers: MCP setup, plugin configuration, usage

## Recommended Reading Order

### For New Users
1. Start with [QUICK_START.md](QUICK_START.md) to get Rustbot running
2. If using MCP features, see [MCP_QUICKSTART.md](MCP_QUICKSTART.md)

### For New Contributors
1. Read [QUICK_START.md](QUICK_START.md) to understand basic usage
2. Review [QUICK_START_REFACTORING.md](QUICK_START_REFACTORING.md) for refactoring workflow
3. Check [../architecture/](../architecture/) for architectural context

### For Feature Development
1. Understand basics from [QUICK_START.md](QUICK_START.md)
2. Review relevant feature guide (e.g., [MCP_QUICKSTART.md](MCP_QUICKSTART.md))
3. Consult [../architecture/planning/](../architecture/planning/) for integration points

## Related Resources

- **[Architecture](../architecture/)** - System architecture and design patterns
- **[QA](../qa/)** - Testing guides and quality assurance
- **[Progress](../progress/)** - Implementation session logs
- **[Development Guide](../../DEVELOPMENT.md)** - Comprehensive development guide (root level)

## Contributing New Guides

When creating new guides:
1. Use clear, concise language
2. Include code examples where applicable
3. Add estimated reading time
4. Specify target audience
5. Follow existing guide structure
6. Update this README with the new guide
7. Add cross-references to related documentation

## Guide Writing Template

```markdown
# [Feature Name] Quick Start

## Overview
Brief description of what this guide covers.

## Prerequisites
- Required knowledge
- Required tools/dependencies

## Step 1: [First Step]
Detailed instructions...

## Step 2: [Second Step]
Detailed instructions...

## Verification
How to verify the feature is working correctly.

## Troubleshooting
Common issues and solutions.

## Next Steps
- Related guides
- Advanced topics
```

## Quick Access by Goal

- **Get Rustbot running** → [QUICK_START.md](QUICK_START.md)
- **Work on refactoring** → [QUICK_START_REFACTORING.md](QUICK_START_REFACTORING.md)
- **Set up MCP plugins** → [MCP_QUICKSTART.md](MCP_QUICKSTART.md)
- **Learn development workflow** → [../../DEVELOPMENT.md](../../DEVELOPMENT.md)
