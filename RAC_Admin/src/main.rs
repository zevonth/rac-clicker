use rsa::{RsaPrivateKey, RsaPublicKey};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::io::{self, Write};
use std::fs;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use aes_gcm::aead::Aead;
use time::{Duration, OffsetDateTime};
use base64::{Engine as _, engine::general_purpose};
use rand::Rng;
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};

const PRIVATE_KEY: &str = "
-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDQFUAg4XUvKh3B
Rne/oEjgO0Ls8FndMzD2e6c/ynySEcEDBanYFwFNo5EguZ+OmMd4FWI0aB6BhkJS
xX7RxTBC+jwuYHa1zGegX5hlRsTOdvFggPz/e1o6MzoHYxBfNGH1wY2kJrAfKwSb
y3DVSQNE9emyKAw17XTlw79m7/3YktN64/Vp45E93ygbk9TRmb1CfahLYn+WAWQZ
Ypoi5EoKDddCcBpAcF1Fr0/TqPimLoXoNMS6TkyiNmNfS5HtNNaDdbmXR0p7yUlN
I/jJwj3W4EvGUFwcmhZhyfRUF0KztlIgO1yzDYroq44vg9GYoWgm/bFgze6e9isv
Kj2lg/OfAgMBAAECggEBALYthMHE4rXyZ66ppXnuOR+ogsWzANp7USjbxehBvaKd
TKD6umLocUmqJQvDuIA+HpVyE1LSvbKk+zhAlPHPdJuPPlVUO1qbpTZxu5kfxnsF
A/t7swVy1+IQq4OAJftUf1eMqBfJj3UaUqScDyONEwGzU3GZQmeMiEYJhW/4OgT7
ouQbSqQfYKJMOdc8URue0VgLOn3qxN4AMCqlm2sBroZfM1ZdLDPR52codgF2mFkn
2ks/aMa57Z+iIFl3IUj/AuqUzGUiO6jk6/hdiTKM+awKE8XRfcSqZip/bpPHDCHG
4zADRcWtB2IoRhWYJYtCrJ2X/poiV8PM++s5F5b8QQECgYEA20jInQpKO3Efuhjq
CdqumpvVh+JmiG++jVPR4AO98hxhUL30GaJht81Vs6ERN4rGISe2OQgC9NSeO3uo
ndNEGNGxuxdWsJn2trF91yao6CWF5gLKYwphiYJWC9L7RvLiRO9o+fEZrdldSOsV
pA3LN3SrvyPYYiQoV6CZ0oIuU18CgYEA8uxYAuHQ+jQqLaYwyqcJoX3ZGF13s3jL
9latPtyWDBSNy4Om+XKVH6kpNsjt9oXe3IrEwQMwi3iJ7FramIMgb7KkL+0WLGpK
6H9tQ+k56+utQotGNYIC0crS7Wyxy/lslO/Cl6+xJfm9XUWCUlOlLNl0c6JQg1wG
Nr9FlZFgh8ECgYBh62lx/tNRId3KCzAPQHCxp569dBLXIBcWIvTYNLOQNq4aEQi8
na9HFVEMyoLWq4h367TKWNKBI6SY6UpNV7bT4mecJPrYF5h80ltgROot5/uXz20y
tsMBVYs48ni0oOb7Y3EcE1alNCgc1KUwQdyaIeZDjy0j/gNpxdSKlQk8vwKBgFTc
N0qadBRTkMBto/HHNFgTzHj0fdJWSnn9gXvKNyh1LauAjB3r5yguQdV/j5Fk5puI
Zn8+jADM1PGaM26/r98VOsH7Qpm1cAGUMildGfzOUbJDUP10M2PyRIjoRZwJv+Kv
dvziRTIM8hfZJlN4IOVv+XxDoXih9xG886PyKxSBAoGBANWUWCsx2d4SggmhueFp
V5faeoRrITLT+dlIUqfy07e4y6jCB/Ey+ccCPlgxjWrH5Cys+qWpiBa7YpvEX8kU
vxPyRj7tIB42wMK0LYcrzcF8e1k8gX8/fTERbflSp+c/HsuXjRLMlhnRD3wNLioR
pNsglDk2eyeY+xIsV6Eb9glH
-----END PRIVATE KEY-----
";

const PUBLIC_KEY: &str = "
-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA0BVAIOF1LyodwUZ3v6BI
4DtC7PBZ3TMw9nunP8p8khHBAwWp2BcBTaORILmfjpjHeBViNGgegYZCUsV+0cUw
Qvo8LmB2tcxnoF+YZUbEznbxYID8/3taOjM6B2MQXzRh9cGNpCawHysEm8tw1UkD
RPXpsigMNe105cO/Zu/92JLTeuP1aeORPd8oG5PU0Zm9Qn2oS2J/lgFkGWKaIuRK
Cg3XQnAaQHBdRa9P06j4pi6F6DTEuk5MojZjX0uR7TTWg3W5l0dKe8lJTSP4ycI9
1uBLxlBcHJoWYcn0VBdCs7ZSIDtcsw2K6KuOL4PRmKFoJv2xYM3unvYrLyo9pYPz
nwIDAQAB
-----END PUBLIC KEY-----
";

const ENCRYPTION_KEY: &str = "Mtydz8l67yxJwIuvw9IRpjRgFNcd1qAsaMVNmhVQOeQ=";

/*
lazy_static! {
    static ref ENCRYPTION_KEY: [u8; 32] = {
        let key = generate_encryption_key();

        // print key
        println!("Generated Key (base64): {}", general_purpose::STANDARD.encode(&key));
        key
    };
}
*/

#[derive(Debug, Serialize, Deserialize)]
struct LicenseInfo {
    machine_id: String,
    expires_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct License {
    info: LicenseInfo,
    signature: String,
}

fn encrypt_license_data(data: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let key_bytes = general_purpose::STANDARD.decode(ENCRYPTION_KEY)?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let mut rng = rand::rng();
    let mut nonce_bytes = [0u8; 12];
    rng.fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let encrypted = cipher.encrypt(nonce, data.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    let mut final_data = Vec::with_capacity(12 + encrypted.len());
    final_data.extend_from_slice(&nonce_bytes);
    final_data.extend(encrypted);

    Ok(final_data)
}
/*
fn generate_encryption_key() -> [u8; 32] {
    let mut rng = rand::rng();
    let mut key = [0u8; 32];
    rng.fill(&mut key);
    key
}
*/
fn load_private_key() -> Result<RsaPrivateKey, Box<dyn std::error::Error>> {
    Ok(RsaPrivateKey::from_pkcs8_pem(PRIVATE_KEY)?)
}

fn create_license(
    private_key: &RsaPrivateKey,
    machine_id: &str,
    days_valid: u16,
) -> Result<License, Box<dyn std::error::Error>> {
    let expires_at = OffsetDateTime::now_utc() + Duration::days(days_valid as i64);

    let info = LicenseInfo {
        machine_id: machine_id.to_string(),
        expires_at: expires_at.unix_timestamp(),
    };

    let info_bytes = serde_json::to_vec(&info)?;
    let mut hasher = Sha256::new();
    hasher.update(&info_bytes);
    let hash = hasher.finalize();

    let signature = private_key.sign(
        rsa::Pkcs1v15Sign::new::<Sha256>(),
        &hash
    )?;

    let encoded_signature = general_purpose::STANDARD.encode(signature);

    Ok(License {
        info,
        signature: encoded_signature,
    })
}

fn decrypt_license_data(encrypted_data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    if encrypted_data.len() < 12 {
        return Err("Invalid encrypted data length".into());
    }

    let decoded_key = general_purpose::STANDARD.decode(ENCRYPTION_KEY)?;
    let key = Key::<Aes256Gcm>::from_slice(&decoded_key);
    let cipher = Aes256Gcm::new(key);

    let nonce = Nonce::from_slice(&encrypted_data[..12]);
    let ciphertext = &encrypted_data[12..];

    let decrypted = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    String::from_utf8(decrypted)
        .map_err(|e| format!("Invalid UTF-8: {}", e).into())
}

fn validate_license(license_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let encrypted_data = fs::read(license_path)?;
    let license_data = decrypt_license_data(&encrypted_data)?;
    let license: License = serde_json::from_str(&license_data)?;

    let public_key = RsaPublicKey::from_public_key_pem(PUBLIC_KEY)?;

    let info_bytes = serde_json::to_vec(&license.info)?;
    let mut hasher = Sha256::new();
    hasher.update(&info_bytes);
    let hash = hasher.finalize();

    let signature_bytes = general_purpose::STANDARD.decode(&license.signature)?;

    match public_key.verify(
        rsa::Pkcs1v15Sign::new::<Sha256>(),
        &hash,
        &signature_bytes
    ) {
        Ok(_) => {
            let now = OffsetDateTime::now_utc().unix_timestamp();
            if now > license.info.expires_at {
                println!("License has expired!");
                Ok(false)
            } else {
                Ok(true)
            }
        },
        Err(_) => Ok(false),
    }
}

fn print_menu() {
    println!("\nLicense Management System");
    println!("1. Generate License");
    println!("2. Validate License");
    println!("3. Exit");
    print!("Select an option: ");
    io::stdout().flush().unwrap();
}

fn generate_license_flow() -> Result<(), Box<dyn std::error::Error>> {
    let private_key = load_private_key()?;

    print!("Enter Machine ID: ");
    io::stdout().flush()?;
    let mut machine_id = String::new();
    io::stdin().read_line(&mut machine_id)?;
    let machine_id = machine_id.trim();

    print!("Enter Days Valid: ");
    io::stdout().flush()?;
    let mut days_valid = String::new();
    io::stdin().read_line(&mut days_valid)?;
    let days_valid: u16 = days_valid.trim().parse()?;

    let license = create_license(&private_key, machine_id, days_valid)?;
    let license_json = serde_json::to_string_pretty(&license)?;

    let encrypted_data = encrypt_license_data(&license_json)?;

    let file_name = format!("{}.license", machine_id);
    fs::write(&file_name, encrypted_data)?;

    println!("License generated successfully: {}", file_name);
    Ok(())
}

fn validate_license_flow() -> Result<(), Box<dyn std::error::Error>> {
    print!("Enter license file path: ");
    io::stdout().flush()?;
    let mut path = String::new();
    io::stdin().read_line(&mut path)?;
    let path = path.trim();

    match validate_license(path) {
        Ok(true) => println!("License is valid!"),
        Ok(false) => println!("License is invalid or expired!"),
        Err(e) => println!("Error validating license: {}", e),
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    loop {
        print_menu();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;

        match choice.trim() {
            "1" => generate_license_flow()?,
            "2" => validate_license_flow()?,
            "3" => {
                println!("Goodbye!");
                break;
            }
            _ => println!("Invalid option, please try again."),
        }
    }
    Ok(())
}
