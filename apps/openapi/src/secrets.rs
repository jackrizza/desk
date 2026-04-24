use database::Database;

pub trait TraderSecretStore {
    fn normalize_openai_key(&self, key: &str) -> Result<String, String>;
}

pub struct DatabaseTraderSecretStore<'a> {
    #[allow(dead_code)]
    database: &'a Database,
}

impl<'a> DatabaseTraderSecretStore<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }
}

impl TraderSecretStore for DatabaseTraderSecretStore<'_> {
    fn normalize_openai_key(&self, key: &str) -> Result<String, String> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err("openai_api_key must be non-empty".to_string());
        }

        // TODO(security): encrypt before storage or delegate to a secret manager/Key Vault.
        Ok(trimmed.to_string())
    }
}
