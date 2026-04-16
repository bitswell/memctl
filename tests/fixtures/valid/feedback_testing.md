---
name: Testing preferences
description: Always use integration tests over mocks
type: feedback
tags:
  - testing
  - workflow
---

Use real databases in tests. Mocks hide bugs.

**Why:** Prior incident where mocked tests passed but production failed.

**How to apply:** Any test touching the database layer should use a real test database.
