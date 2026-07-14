# Access Control Evidence - 2026-07-09

## MFA Status
./.specs/01_research/test_vectors/test_vectors_auth.toml:    context.authentication.mfa == true
./.specs/01_research/test_vectors/test_vectors_auth.toml:request_context = """{"authentication": {"mfa": true}, "network": {"ip": "10.0.1.50"}}"""
./.specs/02_architecture/interface_contracts/interface_contracts_auth.toml:    context = "Context (record: {ip, mfa, time, ...})",

## RBAC Configuration
No RBAC config found

## User Management
./crates/server/Cargo.toml:ferro-server-user-mgmt = { path = "../server-user-mgmt" }
./crates/server-user-mgmt/Cargo.toml:name = "ferro-server-user-mgmt"
./.specs/01_research/domain_constraints/domain_constraints_auth.toml:rationale = "Short-lived tokens reduce the window for token theft and replay. 10 minutes balances security with user experience (no excessive re-authentication). Configurable per deployment."
./.specs/01_research/domain_constraints/domain_constraints_auth.toml:rationale = "Cedar security best practices: use unique, immutable, non-recyclable identifiers. If a user leaves and their username is reassigned, policies referencing the old username would grant access to the new user. OIDC sub claims from compliant providers (Keycloak, Okta) are UUIDs."
./.specs/01_research/domain_constraints/domain_constraints_auth.toml:enforcement = "Token validator maps OIDC sub claim directly to Cedar User::\"<sub>\" UID; no username-based mapping"
./.specs/01_research/domain_constraints/domain_constraints_storage.toml:max_concurrent_presign_per_user = 10
./.specs/01_research/domain_constraints/domain_constraints_webdav.toml:rationale = "Office save-as behavior; user expects near-instantaneous copy for typical document sizes"
./.specs/01_research/test_vectors/test_vectors_cas.toml:user_path = "/documents/hello.txt"
./.specs/01_research/test_vectors/test_vectors_cas.toml:user_path = "/empty/zero.txt"
./.specs/01_research/test_vectors/test_vectors_cas.toml:user_path_1 = "/docs/report_v1.txt"
./.specs/01_research/test_vectors/test_vectors_cas.toml:user_path_2 = "/archive/report_v1_backup.txt"
./.specs/01_research/test_vectors/test_vectors_cas.toml:user_path = "/docs/invariant.txt"
./.specs/01_research/test_vectors/test_vectors_cas.toml:user_path = "/docs/corrupted.txt"
./.specs/01_research/test_vectors/test_vectors_cas.toml:user_path = "/large/video.mp4"
./.specs/01_research/test_vectors/test_vectors_cas.toml:user_path = "/shared/document.pdf"
./.specs/01_research/test_vectors/test_vectors_cas.toml:user_path = "/shared/old.pdf"
./.specs/01_research/test_vectors/test_vectors_cas.toml:user_path = "/uploads/new_file.txt"
./.specs/01_research/test_vectors/test_vectors_cas.toml:user_path = "/docs/duplicate.txt"
./.specs/01_research/test_vectors/test_vectors_cas.toml:user_path = "/admin/secrets.txt"
./.specs/01_research/test_vectors/test_vectors_cas.toml:user_path = "/docs/file.txt"
No user config found
