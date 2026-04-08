# Product Guidelines: ripweb

These guidelines define the stylistic, UX, and technical standards for `ripweb`.

## Prose & Communication
- **Tone**: Technical & Concise. Communication should be direct, clear, and efficient.
- **Output**: Tool output should prioritize information density over conversational fluff.

## User Experience (UX)
- **Philosophy**: CLI-First / Scriptable & Unix Philosophy.
- **Integration**: Designed for easy integration into automated workflows and pipelines.
- **Conventions**: Follow standard Unix conventions for pipes, signals, and exit codes.
- **Output Routing**: Ensure stdout/stderr are used correctly for content vs. metadata/errors.

## Extraction & Markdown Quality
- **Fidelity**: Maintain Structural Fidelity (tables, lists, headers) by default.
- **Optimization Levels**: Implement different optimization levels to condense content (e.g., from full text to summarized paragraphs/headers).
- **Validation**: Validate extraction quality against real token counter libraries (e.g., `tiktoken`).
- **Standardization**: All output should be consistent, GFM-compliant Markdown.

## Error Handling
- **Unix Philosophy**: Adhere to professional Unix standards for error reporting.
- **LLM Compatibility**: Errors should be structured and clear for consumption by LLMs in tool-calling contexts.
- **Exit Codes**: Use meaningful, standardized exit codes to indicate failure reasons.
