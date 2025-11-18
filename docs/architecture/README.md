# Architecture Documentation

## Purpose
This directory contains all architecture-related documentation for the Rustbot project, including best practices, planning documents, implementation details, and visual diagrams.

## Directory Structure

### 📚 Best Practices (`best-practices/`)
Rust and architecture best practices specific to this project.
- **[RUST_ARCHITECTURE_BEST_PRACTICES.md](best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md)** - Comprehensive guide to Rust architecture patterns and principles

### 📋 Planning (`planning/`)
Strategic planning and research documents for the architecture refactoring.
- **[RUSTBOT_REFACTORING_PLAN.md](planning/RUSTBOT_REFACTORING_PLAN.md)** - Master refactoring plan with phases and timeline
- **[REFACTORING_CHECKLIST.md](planning/REFACTORING_CHECKLIST.md)** - Detailed checklist for tracking refactoring progress
- **[ARCHITECTURE_RESEARCH_SUMMARY.md](planning/ARCHITECTURE_RESEARCH_SUMMARY.md)** - Research findings and architectural decisions

### ⚙️ Implementation (`implementation/`)
Implementation summaries, prototypes, and test results.
- **[PHASE1_IMPLEMENTATION_SUMMARY.md](implementation/PHASE1_IMPLEMENTATION_SUMMARY.md)** - Phase 1 implementation details and outcomes
- **[PROTOTYPE_REFACTORING.md](implementation/PROTOTYPE_REFACTORING.md)** - Prototype implementation guide
- **[PROTOTYPE_TEST_RESULTS.md](implementation/PROTOTYPE_TEST_RESULTS.md)** - Test results from prototype implementation
- **[REFACTORING_PROTOTYPE_SUMMARY.md](implementation/REFACTORING_PROTOTYPE_SUMMARY.md)** - Comprehensive prototype summary

### 📊 Diagrams (`diagrams/`)
Visual architecture diagrams and data flow charts.
- **[README.md](diagrams/README.md)** - Diagrams directory index
- **[ARCHITECTURE_DIAGRAMS.md](diagrams/ARCHITECTURE_DIAGRAMS.md)** - System architecture diagrams
- **[DATA_FLOW.md](diagrams/DATA_FLOW.md)** - Data flow diagrams
- **[REFACTORING_TIMELINE.md](diagrams/REFACTORING_TIMELINE.md)** - Visual refactoring timeline
- **[DIAGRAM_CREATION_SUMMARY.md](diagrams/DIAGRAM_CREATION_SUMMARY.md)** - Guide to creating diagrams
- **[MERMAID_EXAMPLES.md](diagrams/MERMAID_EXAMPLES.md)** - Mermaid diagram examples

## Recommended Reading Order

### For New Developers
1. Start with [RUST_ARCHITECTURE_BEST_PRACTICES.md](best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md) to understand architectural principles
2. Review [ARCHITECTURE_DIAGRAMS.md](diagrams/ARCHITECTURE_DIAGRAMS.md) for visual overview
3. Read [RUSTBOT_REFACTORING_PLAN.md](planning/RUSTBOT_REFACTORING_PLAN.md) to understand current direction

### For Contributors to Refactoring
1. Read [RUSTBOT_REFACTORING_PLAN.md](planning/RUSTBOT_REFACTORING_PLAN.md) for overall strategy
2. Review [REFACTORING_CHECKLIST.md](planning/REFACTORING_CHECKLIST.md) to see progress
3. Study [PROTOTYPE_REFACTORING.md](implementation/PROTOTYPE_REFACTORING.md) for implementation details
4. Check [PROTOTYPE_TEST_RESULTS.md](implementation/PROTOTYPE_TEST_RESULTS.md) for validation

### For Architects
1. Review [ARCHITECTURE_RESEARCH_SUMMARY.md](planning/ARCHITECTURE_RESEARCH_SUMMARY.md) for research findings
2. Study [RUST_ARCHITECTURE_BEST_PRACTICES.md](best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md) for patterns
3. Examine [DATA_FLOW.md](diagrams/DATA_FLOW.md) for system interactions

## Related Resources

- **[Guides](../guides/)** - Quick start and usage guides
- **[QA](../qa/)** - Testing and quality assurance documentation
- **[Progress](../progress/)** - Session logs and implementation progress
- **[Fixes](../fixes/)** - Bug fixes and issue resolutions

## Maintenance

When updating architecture documentation:
1. Keep diagrams in sync with code changes
2. Update the refactoring checklist as tasks complete
3. Add new patterns to best practices guide
4. Document architectural decisions in progress logs
5. Update this README when adding new documents

## Quick Access by Task

- **Understanding current architecture** → [ARCHITECTURE_DIAGRAMS.md](diagrams/ARCHITECTURE_DIAGRAMS.md)
- **Planning refactoring work** → [REFACTORING_CHECKLIST.md](planning/REFACTORING_CHECKLIST.md)
- **Implementing changes** → [PROTOTYPE_REFACTORING.md](implementation/PROTOTYPE_REFACTORING.md)
- **Creating diagrams** → [DIAGRAM_CREATION_SUMMARY.md](diagrams/DIAGRAM_CREATION_SUMMARY.md)
- **Following best practices** → [RUST_ARCHITECTURE_BEST_PRACTICES.md](best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md)
