---
name: data-extraction
domain: research
version: 1
trigger_patterns:
  - "extract data"
  - "structured extraction"
  - "pull information"
  - "data mining"
applicable_agents:
  - deep-researcher
  - competitive-analyst
---
# Data Extraction

## Steps
1. Define schema: what entities, attributes, and relationships need to be extracted
2. Identify source structure: tables, lists, paragraphs, PDFs, HTML
3. Extract systematically: manual curation vs automated (regex, NLP, scraping)
4. Validate extraction: spot-check against original, ensure no data loss
5. Standardize formats: dates, currencies, units, names into consistent representations
6. Document extraction rules and assumptions for reproducibility

## Examples
- Company data: name, founded year, funding rounds, headcount, HQ location from Crunchbase
- Product specs: dimensions, weight, price, ratings from e-commerce listings
- Financial tables: revenue, profit, margins from annual report PDFs

## Anti-patterns
- Extracting without a predefined schema (you'll miss important fields)
- Not handling missing values explicitly
- Mixing data types in a single field
- Losing provenance information (which source did each value come from?)
