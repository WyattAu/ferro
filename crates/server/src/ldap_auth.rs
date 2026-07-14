#[derive(Debug)]
pub struct LdapConfig {
    pub url: String,
    pub bind_dn: String,
    pub bind_password: String,
    pub user_search_base: String,
    pub user_filter: String,
    pub email_attribute: String,
    pub display_name_attribute: String,
    /// Base DN for searching groups. When set, the user's LDAP groups are queried
    /// and matched against `group_role_map` to determine the user role.
    pub group_search_base: Option<String>,
    /// LDAP filter for group search. `{dn}` is replaced with the user's DN.
    /// Default: `(member={dn})`
    pub group_filter: Option<String>,
    /// Mapping from LDAP group CN to Ferro role. Example:
    /// `{"ferro-admins": "Admin", "ferro-readonly": "ReadOnly"}`
    /// Unmatched users get the default role (User).
    pub group_role_map: std::collections::HashMap<String, String>,
}

#[derive(Debug)]
pub struct LdapError {
    pub message: String,
}

impl std::fmt::Display for LdapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LDAP error: {}", self.message)
    }
}

impl LdapError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { message: msg.into() }
    }
}

pub async fn ldap_authenticate(
    config: &LdapConfig,
    username: &str,
    password: &str,
) -> Result<crate::users::User, LdapError> {
    let settings = ldap3::LdapConnSettings::new().set_conn_timeout(std::time::Duration::from_secs(5));

    let (_conn, mut ldap) = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        ldap3::LdapConnAsync::with_settings(settings, &config.url),
    )
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => return Err(LdapError::new(format!("LDAP connection failed: {}", e))),
        Err(_) => return Err(LdapError::new("LDAP connection timed out (5s)")),
    };

    let bind_result = match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        ldap.simple_bind(&config.bind_dn, &config.bind_password),
    )
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => return Err(LdapError::new(format!("LDAP service bind error: {}", e))),
        Err(_) => return Err(LdapError::new("LDAP bind timed out (10s)")),
    };

    if bind_result.success().is_err() {
        return Err(LdapError::new("LDAP service bind failed"));
    }

    let filter = config.user_filter.replace("{username}", username);
    let search_result = match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        ldap.search(
            &config.user_search_base,
            ldap3::Scope::Subtree,
            &filter,
            vec![&config.email_attribute, &config.display_name_attribute, "uid"],
        ),
    )
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => return Err(LdapError::new(format!("LDAP search failed: {}", e))),
        Err(_) => return Err(LdapError::new("LDAP search timed out (10s)")),
    };

    let (entries, _search_res) = match search_result.success() {
        Ok(result) => result,
        Err(_) => return Err(LdapError::new("LDAP search returned no results")),
    };

    let entry = entries
        .into_iter()
        .next()
        .ok_or_else(|| LdapError::new("User not found in LDAP"))?;

    let search_entry = ldap3::SearchEntry::construct(entry);
    let user_dn = search_entry.dn;

    if let Err(e) = ldap.unbind().await {
        tracing::warn!("LDAP unbind failed: {}", e);
    }

    let settings2 = ldap3::LdapConnSettings::new().set_conn_timeout(std::time::Duration::from_secs(5));
    let (_conn2, mut ldap_user) = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        ldap3::LdapConnAsync::with_settings(settings2, &config.url),
    )
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => return Err(LdapError::new(format!("LDAP reconnection failed: {}", e))),
        Err(_) => return Err(LdapError::new("LDAP user connection timed out (5s)")),
    };

    let user_bind_result = match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        ldap_user.simple_bind(&user_dn, password),
    )
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => return Err(LdapError::new(format!("LDAP user bind error: {}", e))),
        Err(_) => return Err(LdapError::new("LDAP user bind timed out (10s)")),
    };

    if user_bind_result.success().is_err() {
        return Err(LdapError::new("Invalid LDAP credentials"));
    }

    if let Err(e) = ldap_user.unbind().await {
        tracing::warn!("LDAP user unbind failed: {}", e);
    }

    let email = search_entry
        .attrs
        .get(&config.email_attribute)
        .and_then(|v| v.first())
        .cloned()
        .unwrap_or_default();

    let display_name = search_entry
        .attrs
        .get(&config.display_name_attribute)
        .and_then(|v| v.first())
        .cloned()
        .unwrap_or_else(|| username.to_string());

    // Resolve user role from LDAP groups if group mapping is configured
    let role = if config.group_search_base.is_some() && !config.group_role_map.is_empty() {
        resolve_role_from_groups(config, &user_dn).await
    } else {
        crate::users::UserRole::User
    };

    Ok(crate::users::User {
        id: uuid::Uuid::new_v4().to_string(),
        username: username.to_string(),
        display_name,
        email,
        role,
        created_at: chrono::Utc::now(),
        last_login: Some(chrono::Utc::now()),
        status: crate::users::UserStatus::Active,
        storage_quota_bytes: None,
        storage_used_bytes: 0,
        is_ldap: true,
        password_hash: None,
        totp_secret: None,
        totp_enabled: false,
    })
}

/// Query LDAP groups for a user and map them to a Ferro role.
///
/// Connects with the service account, searches for groups containing the user DN,
/// then matches group CNs against the configured `group_role_map`.
/// Returns the highest-privilege matching role, or `UserRole::User` as default.
async fn resolve_role_from_groups(config: &LdapConfig, user_dn: &str) -> crate::users::UserRole {
    let group_base = match &config.group_search_base {
        Some(b) => b,
        None => return crate::users::UserRole::User,
    };

    let group_filter = config
        .group_filter
        .as_deref()
        .unwrap_or("(member={dn})")
        .replace("{dn}", user_dn);

    let settings = ldap3::LdapConnSettings::new().set_conn_timeout(std::time::Duration::from_secs(5));

    let (_conn, mut ldap) = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        ldap3::LdapConnAsync::with_settings(settings, &config.url),
    )
    .await
    {
        Ok(Ok(result)) => result,
        _ => {
            tracing::warn!("LDAP group lookup: connection failed");
            return crate::users::UserRole::User;
        }
    };

    if ldap.simple_bind(&config.bind_dn, &config.bind_password).await.is_err() {
        tracing::warn!("LDAP group lookup: service bind failed");
        return crate::users::UserRole::User;
    }

    let search_result = match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        ldap.search(group_base, ldap3::Scope::Subtree, &group_filter, vec!["cn"]),
    )
    .await
    {
        Ok(Ok(result)) => result,
        _ => {
            tracing::warn!("LDAP group lookup: search failed");
            let _ = ldap.unbind().await;
            return crate::users::UserRole::User;
        }
    };

    let _ = ldap.unbind().await;

    let (entries, _) = match search_result.success() {
        Ok(result) => result,
        Err(_) => return crate::users::UserRole::User,
    };

    // Collect group CNs and find the highest-privilege matching role
    let mut resolved_role = crate::users::UserRole::User;
    for entry in entries {
        let se = ldap3::SearchEntry::construct(entry);
        let cn = se.attrs.get("cn").and_then(|v| v.first());
        if let Some(group_name) = cn
            && let Some(role_name) = config.group_role_map.get(group_name)
        {
            let parsed = parse_role(role_name);
            if role_priority(&parsed) > role_priority(&resolved_role) {
                resolved_role = parsed;
            }
        }
    }

    resolved_role
}

/// Parse a role name string into a UserRole.
fn parse_role(name: &str) -> crate::users::UserRole {
    match name.to_lowercase().as_str() {
        "admin" => crate::users::UserRole::Admin,
        "readonly" | "read_only" | "read-only" => crate::users::UserRole::ReadOnly,
        _ => crate::users::UserRole::User,
    }
}

/// Return a priority value for role comparison (higher = more privilege).
fn role_priority(role: &crate::users::UserRole) -> u8 {
    match role {
        crate::users::UserRole::Admin => 3,
        crate::users::UserRole::User => 2,
        crate::users::UserRole::ReadOnly => 1,
        _ => 0,
    }
}
