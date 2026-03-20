---
name: code-devops
description: DevOps patterns — CI/CD, Docker, deployment, monitoring, infrastructure
tools: [bash, read_file, write_file, edit_file]
---

# DevOps Patterns

## CI/CD

- Build → Test → Lint → Deploy (fail fast — cheapest checks first)
- Pin dependency versions in CI for reproducible builds
- Cache build artifacts between runs (cargo registry, node_modules)
- Run tests in parallel where possible
- Branch protection: require passing CI before merge

## Docker

- Multi-stage builds to minimize image size
- Use specific base image tags (not `latest`)
- One process per container
- `.dockerignore` to exclude build artifacts, tests, docs
- Health checks for orchestrator integration

## Deployment

- Blue/green or canary deployments for zero-downtime
- Feature flags for gradual rollout
- Database migrations run BEFORE application deployment
- Rollback plan for every deployment
- Immutable infrastructure — replace, don't patch

## Monitoring

- The four golden signals: latency, traffic, errors, saturation
- Structured logging (JSON) for machine parsing
- Distributed tracing for microservices (OpenTelemetry)
- Alert on symptoms (error rate), not causes (CPU usage)
- Dashboard for each service: health, throughput, latency percentiles

## Infrastructure as Code

- Terraform, Pulumi, or CloudFormation for cloud resources
- Version control all infrastructure definitions
- Plan before apply — review diffs
- State management: remote state with locking

## Security

- Least privilege for all service accounts
- Rotate secrets regularly; use a vault (HashiCorp Vault, AWS Secrets Manager)
- Network segmentation — services only talk to what they need
- Dependency scanning in CI (Dependabot, Snyk)

## Git Workflow

- Short-lived feature branches merged to main
- Conventional commits for automated changelogs
- Squash merge for clean history, merge commits for preserving context
- Tag releases with semantic versioning

## Patterns Learned

*(This section grows as the agent encounters and solves real DevOps problems)*
