//! Cryptography module implementing Argon2id and XChaCha20-Poly1305
//! for note encryption, plus volatile session key management.

use crate::errors::NodaError;
use std::path::Path;
use std::sync::OnceLock;
use std::collections::HashMap;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};

use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce
};
use rand::RngCore;

/// Volatile in-memory state for cryptographic keys
pub struct VolatileSession {
    pub kek: Option<[u8; 32]>,
    pub dek_cache: HashMap<String, [u8; 32]>, // note_id -> decrypted DEK
    pub last_interaction: DateTime<Utc>,
    pub timeout_setting: String, // "1m", "5m", "15m", "1h", "Until App Closes", "Every Time"
}

pub static SESSION: OnceLock<RwLock<VolatileSession>> = OnceLock::new();

pub fn get_session() -> &'static RwLock<VolatileSession> {
    SESSION.get_or_init(|| {
        RwLock::new(VolatileSession {
            kek: None,
            dek_cache: HashMap::new(),
            last_interaction: Utc::now(),
            timeout_setting: "15m".to_string(),
        })
    })
}

/// Start background clock to purge keys upon timeout
pub fn start_timeout_monitor() {
    tokio::spawn(async {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let mut session = get_session().write().await;
            if session.kek.is_none() {
                continue;
            }
            let now = Utc::now();
            let elapsed = now.signed_duration_since(session.last_interaction);

            let limit = match session.timeout_setting.as_str() {
                "1m" => Duration::minutes(1),
                "5m" => Duration::minutes(5),
                "15m" => Duration::minutes(15),
                "1h" => Duration::hours(1),
                "Every Time" => Duration::zero(),
                _ => Duration::weeks(520), // Practically infinite (Until App Closes)
            };

            if elapsed >= limit {
                session.kek = None;
                session.dek_cache.clear();
                tracing::info!("Volatile session expired. Keys purged.");
            }
        }
    });
}

static MONITOR_INIT: std::sync::Once = std::sync::Once::new();

pub fn ensure_monitor_running() {
    MONITOR_INIT.call_once(|| {
        start_timeout_monitor();
    });
}

pub async fn record_interaction() {
    ensure_monitor_running();
    let mut session = get_session().write().await;
    session.last_interaction = Utc::now();
}

fn default_timeout_setting() -> String {
    "15m".to_string()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VaultConfig {
    pub master_password_hash: String,
    pub kek_salt: String, // Base64 encoded salt for KEK derivation
    #[serde(default = "default_timeout_setting")]
    pub timeout_setting: String,
}

impl VaultConfig {
    pub fn load<P: AsRef<Path>>(vault_path: P) -> Result<Self, NodaError> {
        let path = vault_path.as_ref().join(".noda/vault_config.json");
        if !path.exists() {
            return Err(NodaError::Vault("Vault is not configured with a master password yet.".to_string()));
        }
        let content = std::fs::read_to_string(path).map_err(NodaError::Io)?;
        serde_json::from_str(&content).map_err(|e| NodaError::Vault(e.to_string()))
    }

    pub fn save<P: AsRef<Path>>(&self, vault_path: P) -> Result<(), NodaError> {
        let path = vault_path.as_ref().join(".noda/vault_config.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(NodaError::Io)?;
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| NodaError::Vault(e.to_string()))?;
        std::fs::write(path, content).map_err(NodaError::Io)
    }
}

/// Derive a 256-bit KEK from master password and salt using Argon2id
pub fn derive_kek(password: &str, salt: &[u8]) -> Result<[u8; 32], NodaError> {
    use argon2::{Algorithm, Version, Params};
    let params = Params::default();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut kek = [0u8; 32];
    argon2.hash_password_into(password.as_bytes(), salt, &mut kek)
        .map_err(|e| NodaError::Vault(format!("Argon2 KEK derivation failed: {}", e)))?;

    Ok(kek)
}

/// Encrypt raw bytes with XChaCha20-Poly1305
pub fn encrypt_bytes(key: &[u8; 32], plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), NodaError> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let mut nonce_bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, plaintext)
        .map_err(|e| NodaError::Vault(format!("XChaCha20-Poly1305 encryption failed: {}", e)))?;

    Ok((ciphertext, nonce_bytes.to_vec()))
}

/// Decrypt raw bytes with XChaCha20-Poly1305
pub fn decrypt_bytes(key: &[u8; 32], ciphertext: &[u8], nonce_bytes: &[u8]) -> Result<Vec<u8>, NodaError> {
    let cipher = XChaCha20Poly1305::new(key.into());
    if nonce_bytes.len() != 24 {
        return Err(NodaError::Vault(format!("Invalid nonce length: {} (expected 24)", nonce_bytes.len())));
    }
    let nonce = XNonce::from_slice(nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| NodaError::Vault(format!("XChaCha20-Poly1305 decryption failed: {}", e)))?;

    Ok(plaintext)
}

pub async fn encrypt_note_in_place(note: &mut crate::models::note::Note) -> Result<(), NodaError> {
    let session_lock = get_session().read().await;
    let kek = session_lock.kek.ok_or_else(|| NodaError::Vault("Vault is locked".to_string()))?;
    drop(session_lock);

    // 1. Generate random 256-bit DEK
    let mut dek = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut dek);

    // 2. Encrypt DEK with KEK
    let (dek_ciphertext, dek_nonce_bytes) = encrypt_bytes(&kek, &dek)?;
    
    // 3. Encrypt note body with DEK
    let (body_ciphertext, body_nonce_bytes) = encrypt_bytes(&dek, note.body.as_bytes())?;

    // Prepend the nonce to the body ciphertext and base64-encode it
    let mut body_payload = body_nonce_bytes;
    body_payload.extend_from_slice(&body_ciphertext);
    let body_base64 = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &body_payload);

    // 4. Update note fields
    note.is_encrypted = true;
    note.dek_encrypted = Some(base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &dek_ciphertext));
    note.dek_nonce = Some(base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &dek_nonce_bytes));
    note.body = body_base64;

    // Cache decrypted DEK in memory
    let mut session_write = get_session().write().await;
    session_write.dek_cache.insert(note.id.0.to_string(), dek);

    Ok(())
}

pub async fn decrypt_note_in_place(note: &mut crate::models::note::Note) -> Result<(), NodaError> {
    // 1. Get decrypted DEK
    let dek = get_decrypted_dek(&note.id.0.to_string(), note.dek_encrypted.as_deref(), note.dek_nonce.as_deref()).await?;

    // 2. Decode body base64
    let body_payload = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, &note.body)
        .map_err(|e| NodaError::Vault(format!("Failed to decode encrypted body: {}", e)))?;

    if body_payload.len() < 24 {
        return Err(NodaError::Vault("Encrypted body is too short".to_string()));
    }

    let (body_nonce_bytes, body_ciphertext) = body_payload.split_at(24);

    // 3. Decrypt body
    let decrypted_bytes = decrypt_bytes(&dek, body_ciphertext, body_nonce_bytes)?;
    let decrypted_body = String::from_utf8(decrypted_bytes)
        .map_err(|e| NodaError::Vault(format!("Decrypted body is not valid UTF-8: {}", e)))?;

    // 4. Clear fields
    note.is_encrypted = false;
    note.dek_encrypted = None;
    note.dek_nonce = None;
    note.body = decrypted_body;

    // Remove DEK from memory cache
    let mut session_write = get_session().write().await;
    session_write.dek_cache.remove(&note.id.0.to_string());

    Ok(())
}

pub async fn get_decrypted_dek(note_id: &str, dek_encrypted_opt: Option<&str>, dek_nonce_opt: Option<&str>) -> Result<[u8; 32], NodaError> {
    let session_lock = get_session().read().await;
    if let Some(dek) = session_lock.dek_cache.get(note_id) {
        return Ok(*dek);
    }
    let kek = session_lock.kek.ok_or_else(|| NodaError::Vault("Vault is locked".to_string()))?;
    drop(session_lock);

    let dek_encrypted_str = dek_encrypted_opt.ok_or_else(|| NodaError::Vault("Missing encrypted DEK".to_string()))?;
    let dek_nonce_str = dek_nonce_opt.ok_or_else(|| NodaError::Vault("Missing DEK nonce".to_string()))?;

    let dek_ciphertext = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, dek_encrypted_str)
        .map_err(|e| NodaError::Vault(format!("Failed to decode encrypted DEK: {}", e)))?;
    let dek_nonce_bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, dek_nonce_str)
        .map_err(|e| NodaError::Vault(format!("Failed to decode DEK nonce: {}", e)))?;

    let decrypted_dek_bytes = decrypt_bytes(&kek, &dek_ciphertext, &dek_nonce_bytes)?;
    let mut dek = [0u8; 32];
    if decrypted_dek_bytes.len() != 32 {
        return Err(NodaError::Vault("Decrypted DEK is not 256-bit".to_string()));
    }
    dek.copy_from_slice(&decrypted_dek_bytes);

    // Cache DEK
    let mut session_write = get_session().write().await;
    session_write.dek_cache.insert(note_id.to_string(), dek);

    Ok(dek)
}

pub async fn toggle_note_encryption_state(
    vault_path: &Path,
    db: &crate::database::connection::Database,
    vault_service: &crate::vault::service::VaultService,
    note_id: crate::models::note::NoteId,
) -> Result<(), NodaError> {
    // 1. Read note from disk/db
    let mut note = vault_service.read_note(note_id).await?;
    
    // 2. Toggle state
    if note.is_encrypted {
        // Decrypt it
        decrypt_note_in_place(&mut note).await?;
    } else {
        // Encrypt it
        encrypt_note_in_place(&mut note).await?;
    }
    
    // 3. Write note back to disk
    vault_service.write_note(&note).await?;
    
    // 4. Update SQLite database cache
    let conn = db.conn.lock();
    crate::database::queries::update_note(&conn, &note, &note.file_path)?;
    
    // 5. If encrypted, purge FTS5 index body
    if note.is_encrypted {
        crate::database::queries::purge_note_fts_body(&conn, note.id)?;
    }
    
    // 6. Re-compute absolute file size and XXH3 hash of the encrypted/decrypted file on disk
    let full_path = vault_path.join(&note.file_path);
    let size = std::fs::metadata(&full_path).map_err(NodaError::Io)?.len();
    
    let markdown = note.to_markdown().map_err(|e| NodaError::Vault(e.to_string()))?;
    let raw_hash = xxhash_rust::xxh3::xxh3_64(markdown.as_bytes());
    let hash_hex = format!("{:016x}", raw_hash);
    
    // 7. Update sync_file_states atomically and set is_dirty = 1
    conn.execute(
        "INSERT INTO sync_file_states (path, size, hash, is_dirty, retry_count, sync_error) \
         VALUES (?1, ?2, ?3, 1, 0, NULL) \
         ON CONFLICT(path) DO UPDATE SET \
            size = excluded.size, \
            hash = excluded.hash, \
            is_dirty = 1, \
            retry_count = 0, \
            sync_error = NULL",
        rusqlite::params![note.file_path, size as i64, hash_hex],
    ).map_err(|e| NodaError::Database(e.to_string()))?;
    
    Ok(())
}

pub async fn register_master_password(vault_path: &Path, password: &str) -> Result<(), NodaError> {
    // 1. Generate Argon2id hash of password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash_str = argon2.hash_password(password.as_bytes(), &salt)
        .map_err(|e| NodaError::Vault(format!("Argon2 hashing failed: {}", e)))?
        .to_string();

    // 2. Generate random 16-byte KEK salt
    let mut kek_salt_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut kek_salt_bytes);
    let kek_salt_base64 = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &kek_salt_bytes);

    // 3. Save config
    let config = VaultConfig {
        master_password_hash: hash_str,
        kek_salt: kek_salt_base64,
        timeout_setting: "15m".to_string(),
    };
    config.save(vault_path)?;

    // 4. Derive KEK and initialize session to Unlocked
    let derived = derive_kek(password, &kek_salt_bytes)?;
    
    let mut session = get_session().write().await;
    session.kek = Some(derived);
    session.last_interaction = Utc::now();
    session.timeout_setting = "15m".to_string();
    session.dek_cache.clear();

    Ok(())
}

pub async fn check_and_unlock_session(vault_path: &Path, password: &str) -> Result<bool, NodaError> {
    ensure_monitor_running();
    let config = VaultConfig::load(vault_path)?;
    
    // Verify password hash
    let parsed_hash = PasswordHash::new(&config.master_password_hash)
        .map_err(|e| NodaError::Vault(format!("Invalid Argon2 hash stored: {}", e)))?;
    let argon2 = Argon2::default();
    let matches = argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok();
    
    if matches {
        let salt_bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, &config.kek_salt)
            .map_err(|e| NodaError::Vault(format!("Failed to decode KEK salt: {}", e)))?;
            
        let derived = derive_kek(password, &salt_bytes)?;
        
        let mut session = get_session().write().await;
        session.kek = Some(derived);
        session.last_interaction = Utc::now();
        session.timeout_setting = config.timeout_setting.clone();
        session.dek_cache.clear();
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn set_vault_timeout_setting(vault_path: &Path, timeout: String) -> Result<(), NodaError> {
    let mut config = VaultConfig::load(vault_path)?;
    config.timeout_setting = timeout.clone();
    config.save(vault_path)?;

    let mut session = get_session().write().await;
    session.timeout_setting = timeout;
    session.last_interaction = Utc::now();
    Ok(())
}

pub async fn get_vault_timeout_setting(vault_path: &Path) -> String {
    let session = get_session().read().await;
    // If session is unlocked/has a setting, return it; otherwise check config
    if session.kek.is_some() {
        return session.timeout_setting.clone();
    }
    drop(session);

    if let Ok(config) = VaultConfig::load(vault_path) {
        config.timeout_setting
    } else {
        "15m".to_string()
    }
}

pub async fn is_vault_session_unlocked() -> bool {
    let session = get_session().read().await;
    session.kek.is_some()
}

pub fn is_vault_configured(vault_path: &Path) -> bool {
    vault_path.join(".noda/vault_config.json").exists()
}

pub async fn lock_session_instantly() {
    let mut session = get_session().write().await;
    session.kek = None;
    session.dek_cache.clear();
}

pub async fn change_master_password(
    vault_path: &Path,
    db: &crate::database::connection::Database,
    vault_service: &crate::vault::service::VaultService,
    old_password: &str,
    new_password: &str,
) -> Result<(), NodaError> {
    use argon2::PasswordHasher;
    // 1. Load config and verify old password
    let config = VaultConfig::load(vault_path)?;
    let parsed_hash = PasswordHash::new(&config.master_password_hash)
        .map_err(|e| NodaError::Vault(format!("Invalid Argon2 hash stored: {}", e)))?;
    let argon2 = Argon2::default();
    if argon2.verify_password(old_password.as_bytes(), &parsed_hash).is_err() {
        return Err(NodaError::Vault("Incorrect old password".to_string()));
    }

    // 2. Derive old KEK
    let old_salt_bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, &config.kek_salt)
        .map_err(|e| NodaError::Vault(format!("Failed to decode old KEK salt: {}", e)))?;
    let old_kek = derive_kek(old_password, &old_salt_bytes)?;

    // 3. Derive new KEK
    let mut new_salt_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut new_salt_bytes);
    let new_kek = derive_kek(new_password, &new_salt_bytes)?;
    let new_salt_base64 = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &new_salt_bytes);

    // 4. Fetch all encrypted notes from DB
    let note_ids: Vec<String> = {
        let conn = db.conn.lock();
        let mut stmt = conn.prepare("SELECT id FROM notes WHERE is_encrypted = 1")
            .map_err(|e| NodaError::Database(e.to_string()))?;
        let mapped = stmt.query_map([], |row| row.get(0))
            .map_err(|e| NodaError::Database(e.to_string()))?;
        mapped.filter_map(|r| r.ok()).collect()
    };

    // 5. Re-encrypt DEKs
    for id_str in note_ids {
        let note_id = crate::models::note::NoteId(ulid::Ulid::from_string(&id_str).unwrap());
        let mut note = vault_service.read_note(note_id).await?;
        
        let dek_encrypted_str = note.dek_encrypted.as_deref()
            .ok_or_else(|| NodaError::Vault("Missing encrypted DEK".to_string()))?;
        let dek_nonce_str = note.dek_nonce.as_deref()
            .ok_or_else(|| NodaError::Vault("Missing DEK nonce".to_string()))?;

        let dek_ciphertext = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, dek_encrypted_str)
            .map_err(|e| NodaError::Vault(format!("Failed to decode encrypted DEK: {}", e)))?;
        let dek_nonce_bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, dek_nonce_str)
            .map_err(|e| NodaError::Vault(format!("Failed to decode DEK nonce: {}", e)))?;

        let decrypted_dek_bytes = decrypt_bytes(&old_kek, &dek_ciphertext, &dek_nonce_bytes)?;
        
        let (new_dek_ciphertext, new_dek_nonce_bytes) = encrypt_bytes(&new_kek, &decrypted_dek_bytes)?;

        note.dek_encrypted = Some(base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &new_dek_ciphertext));
        note.dek_nonce = Some(base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &new_dek_nonce_bytes));

        vault_service.write_note(&note).await?;

        let conn = db.conn.lock();
        crate::database::queries::update_note(&conn, &note, &note.file_path)?;
    }

    // 6. Save new config
    let new_salt = SaltString::generate(&mut OsRng);
    let new_hash = argon2.hash_password(new_password.as_bytes(), &new_salt)
        .map_err(|e| NodaError::Vault(format!("Argon2 hashing failed: {}", e)))?
        .to_string();

    let new_config = VaultConfig {
        master_password_hash: new_hash,
        kek_salt: new_salt_base64,
        timeout_setting: config.timeout_setting,
    };
    new_config.save(vault_path)?;

    // 7. Update memory session
    let mut session = get_session().write().await;
    session.kek = Some(new_kek);
    session.last_interaction = Utc::now();
    session.dek_cache.clear();

    Ok(())
}


