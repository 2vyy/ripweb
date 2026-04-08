# Track Specification: Improve generic content selection heuristics and implement initial page-family detection

## Overview
This track focuses on improving the core value proposition of `ripweb`: high-quality Markdown extraction from arbitrary web pages. The goal is to refine the heuristics used for main content selection and introduce the ability to classify pages into "families" (e.g., Article, Product, Documentation) to apply specialized extraction rules.

## Objectives
- **Refine Content Selection**: Improve the candidate scoring logic to better distinguish main content from boilerplate (nav, sidebars, footers).
- **Implement Page-Family Detection**: Create a system to classify pages based on DOM structure, meta tags, and URL patterns.
- **Initial Family Rules**: Implement specialized rendering/extraction rules for at least one new page family (e.g., `Article` or `Docs`).
- **Validation**: Use existing fixtures and `insta` snapshots to verify improvements and ensure no regressions.

## Scope
- `src/extract/candidate.rs`: Scoring logic updates.
- `src/extract/family.rs`: New module for page family detection and classification.
- `src/extract/render.rs`: Updates to handle family-specific rendering.
- `tests/extract_web.rs`: Integration tests for new heuristics and families.
