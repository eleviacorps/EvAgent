---
name: deployment-patterns
domain: coding
version: 1
trigger_patterns:
  - "CI/CD pipeline"
  - "deployment strategy"
  - "infrastructure"
  - "DevOps"
  - "release process"
applicable_agents:
  - architect
  - planner
  - build-error-resolver
---
# Deployment Patterns

## Steps
1. Design CI pipeline: lint → test → build → security scan → artifact publish
2. Design CD pipeline: deploy to staging → integration tests → deploy to production
3. Choose deployment strategy: rolling, blue/green, canary, or feature flags
4. Containerize application with Docker, orchestrate with Kubernetes
5. Implement infrastructure-as-code with Terraform, Pulumi, or CloudFormation
6. Set up monitoring, alerting, and log aggregation (Prometheus, Grafana, ELK)

## Examples
- Blue/Green: maintain two identical environments, switch traffic instantly
- Canary: route 5% → 20% → 100% traffic to new version with auto-rollback
- GitOps: ArgoCD syncs cluster state from Git repository

## Anti-patterns
- Manual deployment steps (they will fail under pressure)
- Deploying on Friday afternoon
- No rollback plan or database migration rollback strategy
- Ignoring secrets management — hardcoded credentials in code
