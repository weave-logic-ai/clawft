# Agentic-Flow Documentation

Complete documentation for the agentic-flow platform - AI agent orchestration with 66 specialized agents, 213 MCP tools, ReasoningBank learning memory, and autonomous multi-agent swarms.

## 📚 Documentation Structure

### Core Documentation
- **[CLAUDE.md](CLAUDE.md)** - Claude Code configuration and SPARC development environment setup

### Quick Start & Guides
- **[guides/](guides/)** - User guides, tutorials, and how-to documentation
  - [Quick Start Guide](guides/QUICK-START-v1.7.1.md)
  - [MCP Tools](guides/MCP-TOOLS.md)
  - [Claude Code Integration](guides/CLAUDE-CODE-INTEGRATION.md)
  - [Agent Booster](guides/AGENT-BOOSTER.md)
  - [ReasoningBank](guides/REASONINGBANK.md)
  - [Multi-Model Router](guides/MULTI-MODEL-ROUTER.md)
  - [Deployment Guide](guides/DEPLOYMENT.md)

### Architecture & Design
- **[architecture/](architecture/)** - System architecture and design documents
  - [Executive Summary](architecture/EXECUTIVE_SUMMARY.md)
  - [Package Structure](architecture/PACKAGE_STRUCTURE.md)
  - [QUIC Implementation](architecture/QUIC-IMPLEMENTATION-SUMMARY.md)
  - [Integration Status](architecture/INTEGRATION-STATUS.md)
  - [Improvement Plan](architecture/IMPROVEMENT_PLAN.md)

### Integration Documentation
- **[integration-docs/](integration-docs/)** - Integration guides and implementation summaries
- **[integrations/](integrations/)** - Specific integration implementations
  - [Claude Flow Integration](integrations/CLAUDE_FLOW_INTEGRATION.md)
  - [Flow Nexus Integration](integrations/FLOW-NEXUS-INTEGRATION.md)
  - [FastMCP Integration](integrations/FASTMCP_CLI_INTEGRATION.md)

### Feature Documentation

#### Provider & Router
- **[providers/](providers/)** - Provider fallback and configuration
  - [Provider Fallback Guide](providers/PROVIDER-FALLBACK-GUIDE.md)
  - [Provider Fallback Summary](providers/PROVIDER-FALLBACK-SUMMARY.md)
  - [Landing Page Provider Content](providers/LANDING-PAGE-PROVIDER-CONTENT.md)
- **[router/](router/)** - Multi-model router documentation
  - [Router User Guide](router/ROUTER_USER_GUIDE.md)
  - [Router Config Reference](router/ROUTER_CONFIG_REFERENCE.md)

#### QUIC Transport
- **[quic/](quic/)** - QUIC protocol implementation
  - [QUIC Status](quic/QUIC-STATUS.md)
  - [Implementation Complete](quic/IMPLEMENTATION-COMPLETE-SUMMARY.md)
  - [Final Validation](quic/FINAL-VALIDATION.md)
  - [Performance Validation](quic/PERFORMANCE-VALIDATION.md)

#### ReasoningBank
- **[reasoningbank/](reasoningbank/)** - ReasoningBank learning memory system
  - [Architecture](reasoningbank/REASONINGBANK_ARCHITECTURE.md)
  - [Backends](reasoningbank/REASONINGBANK_BACKENDS.md)
  - [Benchmark Results](reasoningbank/REASONINGBANK-BENCHMARK-RESULTS.md)
  - [Validation](reasoningbank/REASONINGBANK-VALIDATION.md)

#### Supabase Integration
- **[supabase/](supabase/)** - Supabase realtime federation
  - [Quick Start](supabase/QUICKSTART.md)
  - [Implementation Summary](supabase/IMPLEMENTATION-SUMMARY.md)
  - [Test Report](supabase/TEST-REPORT.md)

#### Federation
- **[federation/](federation/)** - Agent federation and deployment
  - [Deployment Validation](federation/DEPLOYMENT-VALIDATION-SUCCESS.md)
  - [Debug Streaming](federation/DEBUG-STREAMING-COMPLETE.md)

### Testing & Validation
- **[testing/](testing/)** - Test documentation and results
  - [Agent System Validation](testing/AGENT-SYSTEM-VALIDATION.md)
  - [Final Testing Summary](testing/FINAL-TESTING-SUMMARY.md)
  - [Regression Test Results](testing/REGRESSION-TEST-RESULTS.md)
- **[validation/](validation/)** - Validation reports and processes
- **[validation-reports/](validation-reports/)** - Detailed validation results
  - [Docker Validation](validation-reports/DOCKER_VALIDATION_RESULTS.md)
  - [No Regressions Confirmed](validation-reports/NO_REGRESSIONS_CONFIRMED.md)
- **[docker-tests/](docker-tests/)** - Docker test configurations

### MCP Validation
- **[mcp-validation/](mcp-validation/)** - Model Context Protocol validation
  - [MCP CLI Validation Report](mcp-validation/MCP-CLI-VALIDATION-REPORT.md)
  - [Implementation Summary](mcp-validation/IMPLEMENTATION-SUMMARY.md)

### Benchmarks & Performance
- **[benchmarks/](benchmarks/)** - Performance benchmarks
  - [QUIC Results](benchmarks/quic-results.md)
  - [Optimization Guide](benchmarks/optimization-guide.md)

### Release Information
- **[releases/](releases/)** - Release notes and GitHub issues
  - [v1.8.13 Release](releases/RELEASE-v1.8.13.md)
  - [v1.5.14 QUIC Transport](releases/v1.5.14-QUIC-TRANSPORT.md)
  - [v1.4.6 ReasoningBank](releases/v1.4.6-reasoningbank-release.md)
- **[version-releases/](version-releases/)** - Version-specific release documentation

### Planning & Research
- **[plans/](plans/)** - Feature plans and roadmaps
  - [QUIC Implementation](plans/QUIC/)
  - [Agent Booster](plans/agent-booster/)
  - [Requesty](plans/requesty/)
- **[quantum-goap/](quantum-goap/)** - Goal-Oriented Action Planning
- **[quantum-research/](quantum-research/)** - Quantum research literature

### Issues & Reports
- **[issues/](issues/)** - Known issues and troubleshooting
  - [Supabase Integration Issue](issues/ISSUE-SUPABASE-INTEGRATION.md)
  - [Xenova Transformers Dependency](issues/ISSUE-xenova-transformers-dependency.md)
- **[reports/](reports/)** - Status reports and completion summaries
- **[reviews/](reviews/)** - Code and implementation reviews

### Archived Documentation
- **[archived/](archived/)** - Historical documentation (kept for reference)
  - Old validation reports
  - Deprecated integration guides
  - Previous version documentation

## 🚀 Quick Links

### Getting Started
1. [Quick Start Guide](guides/QUICK-START-v1.7.1.md)
2. [Claude Code Setup](CLAUDE.md)
3. [Installation Guide](guides/DEPLOYMENT.md)

### Core Features
- [66 Specialized Agents](CLAUDE.md#-available-agents-54-total)
- [213 MCP Tools](guides/MCP-TOOLS.md)
- [ReasoningBank Learning](reasoningbank/README.md)
- [Multi-Model Router](router/README.md)
- [QUIC Transport](quic/QUIC-STATUS.md)

### Integration
- [Claude Flow](integrations/CLAUDE_FLOW_INTEGRATION.md)
- [Flow Nexus](integrations/FLOW-NEXUS-INTEGRATION.md)
- [FastMCP](integrations/FASTMCP_CLI_INTEGRATION.md)
- [Supabase](supabase/README.md)

### Development
- [Architecture Overview](architecture/EXECUTIVE_SUMMARY.md)
- [SPARC Methodology](CLAUDE.md#sparc-workflow-phases)
- [Testing Guide](testing/README.md)

## 📖 Documentation Standards

### File Organization
- Use descriptive filenames in UPPERCASE-WITH-DASHES.md format
- Group related docs in topic-specific directories
- Keep root directory minimal (README, CLAUDE, LICENSE only)

### Content Guidelines
- Start with a clear title and description
- Include table of contents for docs > 200 lines
- Use code blocks with language identifiers
- Add examples and use cases where applicable

### Directory Structure
```
docs/
├── README.md              # This file
├── CLAUDE.md             # Claude Code config
├── LICENSE               # License file
├── architecture/         # System architecture
├── guides/              # User guides
├── integration-docs/    # Integration documentation
├── integrations/        # Specific integrations
├── testing/            # Test documentation
├── releases/           # Release notes
├── issues/             # Known issues
├── providers/          # Provider configuration
├── quic/              # QUIC implementation
├── reasoningbank/     # ReasoningBank docs
├── router/            # Router documentation
├── supabase/          # Supabase integration
└── archived/          # Historical docs
```

## 🔍 Search Tips

### Finding Documentation
- Use `grep -r "keyword" docs/` to search all docs
- Check the appropriate category directory first
- Look in archived/ for historical documentation

### Common Topics
- **Setup**: See guides/ and CLAUDE.md
- **API Reference**: See integration-docs/
- **Troubleshooting**: See issues/
- **Performance**: See benchmarks/
- **Releases**: See releases/ and version-releases/

## 🤝 Contributing to Docs

### Adding New Documentation
1. Identify the appropriate directory
2. Follow naming conventions
3. Include proper frontmatter/headers
4. Update this README index
5. Add cross-references as needed

### Updating Existing Docs
1. Keep archived versions if major changes
2. Update last modified date
3. Maintain backward compatibility links
4. Test all code examples

## 📞 Support

- **GitHub Issues**: https://github.com/ruvnet/agentic-flow/issues
- **Documentation Updates**: Submit PR to docs/ directory
- **Questions**: Check issues/ directory first

---

**Last Updated**: November 6, 2025
**Version**: 1.9.1
