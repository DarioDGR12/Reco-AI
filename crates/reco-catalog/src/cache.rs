use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use reco_core::{Catalog, CatalogSource};

use crate::CatalogError;

pub fn cache_root() -> PathBuf {
    if let Some(custom) = std::env::var_os("RECO_CACHE_DIR") {
        return PathBuf::from(custom);
    }
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("reco")
}

pub fn cache_path() -> PathBuf {
    cache_root().join("catalog-v1.json")
}

pub fn load_cache() -> Option<Catalog> {
    let path = cache_path();
    let data = fs::read_to_string(path).ok()?;
    let mut catalog: Catalog = serde_json::from_str(&data).ok()?;
    if catalog.version != Catalog::VERSION {
        return None;
    }
    catalog.source = CatalogSource::Cache;
    Some(catalog)
}

pub fn save_cache(catalog: &Catalog) -> Result<(), CatalogError> {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| CatalogError::Cache(err.to_string()))?;
    }
    let json = serde_json::to_string_pretty(catalog)
        .map_err(|err| CatalogError::Cache(err.to_string()))?;
    fs::write(path, json).map_err(|err| CatalogError::Cache(err.to_string()))
}

pub fn is_fresh(catalog: &Catalog, ttl_secs: u64) -> bool {
    let Some(fetched) = catalog
        .fetched_at
        .as_deref()
        .and_then(|stamp| stamp.parse::<u64>().ok())
    else {
        return false;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now.saturating_sub(fetched) < ttl_secs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_without_timestamp() {
        let catalog = Catalog::empty(CatalogSource::Seed);
        assert!(!is_fresh(&catalog, 60));
    }
}
