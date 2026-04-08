# Initial Concept
A local, single-binary Rust CLI that fetches web content and converts it to structured Markdown optimized for LLM context windows.

# Product Guide: ripweb

`ripweb` is a local, single-binary Rust CLI that fetches web content and converts it to structured Markdown optimized for LLM context windows.

## Vision
To provide a high-performance, local-first tool for LLM users, developers, and AI researchers to extract clean, high-fidelity content from the web for technical research, RAG systems, and analysis.

## Target Audience
- **Developers**: For code generation and technical research.
- **LLM Power Users**: For feeding web content into LLMs for analysis.
- **AI Researchers**: For building local RAG or search systems.

## Key Goals
- **Performance**: Maximum speed and efficiency for fetching and processing content.
- **Token Efficiency**: Save tokens per page by removing irrelevant content.
- **Fidelity Percentage**: Ensure accuracy of extraction across diverse web domains.

## Core Features
- **Noise Reduction**: Automatic cleaning of boilerplate (nav, footer, ads) to focus on the main content.
- **Navigation Support**: Smart URL normalization and link extraction.
- **Single-Binary Rust CLI**: Easy integration into existing developer workflows.

## Success Metrics
- **Token Efficiency**: Measured by tokens saved per page.
- **Fidelity Percentage**: Measured by accuracy across domains.
- **Latency**: Measured in milliseconds from request to output.
