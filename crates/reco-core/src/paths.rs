use std::path::PathBuf;

/// Data/cache root (`RECO_CACHE_DIR` or platform cache/reco).
pub fn data_dir() -> PathBuf {
    if let Some(custom) = std::env::var_os("RECO_CACHE_DIR") {
        return PathBuf::from(custom);
    }
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("reco")
}

/// Config root (`RECO_CONFIG_DIR` or platform config/reco).
pub fn config_dir() -> PathBuf {
    if let Some(custom) = std::env::var_os("RECO_CONFIG_DIR") {
        return PathBuf::from(custom);
    }
    dirs::config_dir()
        .unwrap_or_else(data_dir)
        .join("reco")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn apis_path() -> PathBuf {
    config_dir().join("apis.json")
}

pub fn clients_dir() -> PathBuf {
    config_dir().join("clients")
}

pub fn db_path() -> PathBuf {
    data_dir().join("reco.db")
}
