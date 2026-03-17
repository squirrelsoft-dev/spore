---
paths:
  - "src/api/**/*.{ts,tsx,js,jsx}"
  - "src/routes/**/*.{ts,tsx,js,jsx}"
  - "src/server/**/*.{ts,tsx,js,jsx}"
  - "app/api/**/*.{ts,tsx,js,jsx}"
---

# API Rules

- All endpoints must include input validation
- Use typed request/response schemas
- Return consistent error formats with appropriate HTTP status codes
- Include rate limiting on public endpoints
- Use parameterized queries or ORM — never raw string interpolation for SQL
