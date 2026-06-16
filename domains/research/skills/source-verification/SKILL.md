---
name: source-verification
domain: research
version: 1
trigger_patterns:
  - "verify source"
  - "check credibility"
  - "fact check"
  - "source reliability"
applicable_agents:
  - deep-researcher
  - literature-reviewer
  - competitive-analyst
---
# Source Verification

## Steps
1. Check source domain authority: .edu/.gov vs .com, publication reputation, author credentials
2. Verify recency: confirm publication date, check for updated versions, look for retractions
3. Cross-reference: find at least 2-3 independent sources supporting the same claim
4. Check methodology: sample size, statistical significance, potential biases
5. Evaluate citations: do cited sources actually support the claims made?
6. Flag conflicts of interest: funding sources, institutional affiliations, known biases

## Examples
- Medical claim: check PubMed for peer-reviewed studies, verify sample size, check for replication
- News article: compare reporting across AP, Reuters, BBC; check original press releases
- Statistics: trace back to original dataset, check methodology, look for cherry-picking

## Anti-patterns
- Relying on a single source for critical claims
- Confusing correlation with causation
- Accepting secondary/tertiary sources without checking the primary source
- Ignoring publication bias (positive results are published more often)
