---
name: security-review
domain: coding
version: 1
trigger_patterns:
  - "security audit"
  - "vulnerability check"
  - "OWASP"
  - "secure coding"
  - "penetration testing"
applicable_agents:
  - code-reviewer
  - architect
---
# Security Review Checklist

## Steps
1. Check authentication: proper password hashing (bcrypt/argon2), session management, MFA
2. Check authorization: least privilege, role-based access control, each endpoint verified
3. Validate inputs: sanitize all user input, parameterized queries (prevent SQL injection)
4. Review data exposure: no secrets in code, proper CORS, HTTPS everywhere
5. Check dependency vulnerabilities: audit outdated packages, known CVEs
6. Review logging: no sensitive data in logs, proper audit trails
7. Rate limiting and brute-force protection on auth endpoints

## Examples
- SQL injection test: `'; DROP TABLE users; --` should be safely escaped
- JWT: use short expiration (15 min), implement refresh tokens, store securely
- CSP headers prevent XSS by restricting script sources

## Anti-patterns
- Rolling your own cryptography
- Trusting client-side validation alone
- Storing passwords in plaintext or with MD5/SHA1
- Exposing internal error messages to users (stack traces in production)
