# Common Criteria Preparation (DC-003)

## TOE (Target of Evaluation) Definition

| Attribute | Value |
|-----------|-------|
| Product Name | Ferro Storage Platform |
| Version | 3.0.0 |
| TOE Boundary | Server binary, CLI, Web UI, Desktop client |
| Network Interfaces | HTTPS (port 8080), WebDAV (port 8080) |
| Users | Administrators, Regular users, Guest users |

### TOE Components

| Component | Description | Interfaces |
|-----------|-------------|------------|
| Ferro Server | Core storage engine | HTTPS, WebDAV, gRPC |
| Ferro CLI | Command-line client | Local process |
| Ferro Web UI | Browser-based management | HTTPS |
| Ferro Desktop | Electron-based client | HTTPS, local IPC |

## Security Function (SF) Requirements

| SF-ID | Security Function | Description |
|-------|------------------|-------------|
| SF-AC | Access Control | OIDC, TOTP, WebAuthn, LDAP, Cedar RBAC |
| SF-AU | Audit | Chain-verified audit log with hash integrity |
| SF-CP | Cryptographic Protection | SHA-256, AES-GCM, ECDSA, TLS 1.3 |
| SF-KM | Key Management | 3-level key hierarchy with rotation |
| SF-DP | Data Protection | WORM storage, retention policies, E2EE |
| SF-IC | Integrity Control | Content hashing, CAS dedup, hash chain verification |
| SF-RC | Recovery | Snapshots, backup/restore, metadata recovery |

## Security Problem Statement

### Threat Agents

| Agent | Motivation | Capability |
|-------|-----------|------------|
| External attacker | Data theft, disruption | Network access, tooling |
| Malicious insider | Data manipulation | Authorized access |
| Compromised client | Credential theft | User-level access |

### Security Objectives

| Objective | Description |
|-----------|-------------|
| O.AUDIT | All security-relevant events are recorded with tamper-evident logs |
| O.ACCESS | Only authorized users can access protected resources |
| O.PROTECT | Data is protected at rest and in transit using strong cryptography |
| O.KEYS | Cryptographic keys are managed securely throughout their lifecycle |
| O.INTEGRITY | Data integrity is maintained and verifiable |
| O.RECOVERY | System can recover from failures and security incidents |

## Assurance Family Requirements (EAL 3+)

| Family | Requirement | Status | Evidence Location |
|--------|-------------|--------|-------------------|
| ADV_ARC | Security architecture description | Partial | `docs/architecture/` |
| ADV_FSP | Functional specification | Partial | `docs/sdk/`, handler documentation |
| ADV_TDS | Technical design | Partial | Blue Papers, crate architecture |
| AGD_OPE | Operational user guidance | Partial | README, `docs/deployment/` |
| AGD_PRE | Preparative procedures | Partial | Docker, Nix, build instructions |
| ALC_CMC | Configuration management | Partial | CI/CD, automated testing |
| ALC_DEL | Delivery procedures | Partial | Docker images, release artifacts |
| ALC_TAT | Tools, techniques, and procedures | Partial | Rust toolchain, formal verification |
| ATE_FUN | Functional testing | Partial | `tests/` (2000+ unit tests, integration) |
| ATE_IND | Independent testing | Partial | Fuzzing, property-based testing |
| AVA_VAN | Vulnerability assessment | Partial | cargo-deny, Trivy, pentest guide |

## Security Function Specification

### SF-AC: Access Control

| Attribute | Value |
|-----------|-------|
| Mechanism | OIDC, TOTP, WebAuthn, LDAP, Cedar RBAC |
| Policy Engine | Cedar (AWS) policy language |
| Authentication Factors | 1FA (password), 2FA (TOTP/WebAuthn) |
| Session Management | JWT with configurable expiry |
| Access Control Model | Role-based with attribute-based extensions |

**Cedar Policy Example:**

```cedar
permit(
  principal == User::"admin",
  action in [Action::"read", Action::"write", Action::"delete"],
  resource
);
```

### SF-AU: Audit

| Attribute | Value |
|-----------|-------|
| Log Format | Chain-verified entries with hash integrity |
| Hash Algorithm | SHA-256 |
| Tamper Evidence | Hash chain verification |
| Events Logged | Auth attempts, CRUD, config changes, errors |
| Log Retention | Configurable with WORM option |

**Audit Entry Structure:**

```rust
struct AuditEntry {
    timestamp: DateTime<Utc>,
    event_type: EventType,
    actor: Actor,
    resource: Resource,
    action: Action,
    result: Result,
    hash: Sha256Digest,
    prev_hash: Option<Sha256Digest>,
}
```

### SF-CP: Cryptographic Protection

| Algorithm | Usage | Standard |
|-----------|-------|----------|
| SHA-256 | Content hashing, audit chain | NIST SP 800-107 |
| AES-GCM-256 | Encryption at rest | NIST SP 800-38D |
| ECDSA (P-256) | Digital signatures | FIPS 186-4 |
| TLS 1.3 | Encryption in transit | RFC 8446 |
| Argon2id | Password hashing | IETF RFC 9106 |

### SF-KM: Key Management

| Level | Key Type | Purpose |
|-------|----------|---------|
| L0 | Master key | Root of trust, wrapped L1 keys |
| L1 | KEK (Key Encryption Key) | Wraps L2 keys |
| L2 | DEK (Data Encryption Key) | Encrypts application data |

**Key Rotation:**

- Master key: Annual rotation (manual)
- KEK: Quarterly rotation (automated)
- DEK: Per-session or per-file (configurable)

### SF-DP: Data Protection

| Feature | Description |
|---------|-------------|
| WORM Storage | Write-once-read-many for compliance |
| Retention Policies | Configurable with legal hold support |
| E2EE | End-to-end encryption for sensitive data |
| Secure Deletion | Cryptographic erasure |

### SF-IC: Integrity Control

| Feature | Description |
|---------|-------------|
| Content Addressable Storage | SHA-256 deduplication |
| Hash Chain Verification | Tamper-evident audit logs |
| Manifest Verification | Signed manifests for bundles |
| Backup Verification | Integrity checks on restore |

### SF-RC: Recovery

| Feature | Description |
|---------|-------------|
| Snapshots | Point-in-time recovery |
| Backup/Restore | Encrypted backups with verification |
| Metadata Recovery | Separate metadata store |
| Disaster Recovery | Tested DR procedures |

## Evaluation Readiness Checklist

- [ ] Security Target (ST) document
- [ ] Protection Profile (PP) selection
- [ ] TOE boundary diagram
- [ ] Security function requirements traceability
- [ ] Assurance family evidence packages
- [ ] Test plan and results
- [ ] Vulnerability assessment results
- [ ] Delivery and operational procedures
- [ ] Administrator guidance
- [ ] User guidance
- [ ] Configuration guide

## Evidence Collection Plan

### ADV_ARC: Security Architecture

| Evidence | Status | Source |
|----------|--------|--------|
| Architecture diagrams | Partial | `docs/architecture/` |
| Data flow diagrams | Missing | Create required |
| Network architecture | Missing | Create required |
| Trust boundaries | Missing | Create required |

### ADV_FSP: Functional Specification

| Evidence | Status | Source |
|----------|--------|--------|
| API specification | Partial | OpenAPI/Swagger |
| Interface descriptions | Partial | `docs/sdk/` |
| Data structures | Partial | Rust types |
| Function mappings | Missing | Map SF to functions |

### ADV_TDS: Technical Design

| Evidence | Status | Source |
|----------|--------|--------|
| High-level design | Partial | Blue Papers |
| Module design | Partial | Crate architecture |
| Detailed design | Missing | Create required |
| Cryptographic design | Partial | `docs/security/` |

### AGD_OPE: Operational User Guidance

| Evidence | Status | Source |
|----------|--------|--------|
| Installation guide | Partial | README |
| Configuration guide | Partial | `docs/deployment/` |
| User manual | Missing | Create required |
| Administration manual | Missing | Create required |

### AGD_PRE: Preparative Procedures

| Evidence | Status | Source |
|----------|--------|--------|
| Build procedures | Partial | Cargo, Nix |
| Deployment procedures | Partial | Docker, Nix |
| Environment setup | Partial | README |
| Verification procedures | Missing | Create required |

### ALC_CMC: Configuration Management

| Evidence | Status | Source |
|----------|--------|--------|
| CM plan | Missing | Create required |
| Build automation | Partial | CI/CD pipelines |
| Version control | Yes | Git repository |
| Change management | Missing | Create required |

### ALC_DEL: Delivery Procedures

| Evidence | Status | Source |
|----------|--------|--------|
| Delivery plan | Missing | Create required |
| Release artifacts | Partial | Docker images, binaries |
| Integrity verification | Partial | Signatures, checksums |
| Distribution channels | Partial | GitHub releases |

### ALC_TAT: Tools, Techniques, and Procedures

| Evidence | Status | Source |
|----------|--------|--------|
| Development tools | Yes | Rust toolchain |
| Testing tools | Partial | cargo test, fuzzing |
| Static analysis | Partial | Clippy, cargo-deny |
| Formal verification | Partial | TODOs for verification |

### ATE_FUN: Functional Testing

| Evidence | Status | Source |
|----------|--------|--------|
| Test plan | Missing | Create required |
| Unit test results | Partial | 2000+ tests |
| Integration tests | Partial | Existing tests |
| Functional test results | Missing | Create required |

### ATE_IND: Independent Testing

| Evidence | Status | Source |
|----------|--------|--------|
| Test strategy | Missing | Create required |
| Fuzzing results | Partial | cargo-fuzz |
| Property-based tests | Partial | proptest |
| Penetration test | Missing | Schedule required |

### AVA_VAN: Vulnerability Assessment

| Evidence | Status | Source |
|----------|--------|--------|
| Vulnerability analysis | Partial | cargo-deny |
| Threat analysis | Missing | Create required |
| Vulnerability scan results | Partial | Trivy |
| Penetration test results | Missing | Schedule required |

## Gap Analysis

### Critical Gaps (Must Complete)

| Gap | Priority | Effort | Owner |
|-----|----------|--------|-------|
| Security Target document | High | 3 weeks | Security team |
| TOE boundary diagram | High | 1 week | Architecture team |
| Threat model | High | 2 weeks | Security team |
| Vulnerability assessment report | High | 2 weeks | Security team |
| Penetration test | High | 2 weeks | External vendor |

### Important Gaps (Should Complete)

| Gap | Priority | Effort | Owner |
|-----|----------|--------|-------|
| Data flow diagrams | Medium | 1 week | Architecture team |
| Configuration management plan | Medium | 1 week | DevOps |
| Delivery procedures | Medium | 1 week | DevOps |
| Test plan and results | Medium | 2 weeks | QA team |

### Nice-to-Have Gaps

| Gap | Priority | Effort | Owner |
|-----|----------|--------|-------|
| Protection Profile selection | Low | 1 week | Security team |
| Formal verification | Low | 4 weeks | Research team |

## Estimated Effort

| Phase | Duration | Activities |
|-------|----------|------------|
| Phase 1: Planning | 2 weeks | ST document, PP selection, TOE boundary |
| Phase 2: Evidence Collection | 4 weeks | Gap closure, evidence packaging |
| Phase 3: Testing | 3 weeks | Functional testing, vulnerability assessment |
| Phase 4: Documentation | 2 weeks | Reports, operational guidance |
| Phase 5: Review | 1 week | Internal review, remediation |

**Total Estimate: 2-3 months for EAL 3 preparation**

## References

- [Common Criteria Portal](https://www.commoncriteriaportal.org/)
- [NIST SP 800-53](https://csrc.nist.gov/publications/detail/sp/800-53/rev-5/final)
- [ISO/IEC 15408](https://www.iso.org/standard/76559.html)
- [Ferro Architecture Documentation](../architecture/)
- [Ferro Security Documentation](../security/)

---

*Document Version: 1.0*
*Last Updated: $(date)*
*Author: Ferro Security Team*
