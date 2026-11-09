use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

pub struct TlsConfig {
    pub cert_pem: Vec<u8>,
    pub key_pem: Vec<u8>,
    pub fingerprint: String,
}

pub fn get_or_create_tls_certs(certs_dir: PathBuf) -> Result<TlsConfig, Box<dyn std::error::Error + Send + Sync>> {
    fs::create_dir_all(&certs_dir)?;
    let cert_path = certs_dir.join("cert.pem");
    let key_path = certs_dir.join("key.pem");
    let fingerprint_path = certs_dir.join("fingerprint.txt");

    if cert_path.exists() && key_path.exists() && fingerprint_path.exists() {
        let cert_pem = fs::read(&cert_path)?;
        let key_pem = fs::read(&key_path)?;
        let fingerprint = fs::read_to_string(&fingerprint_path)?.trim().to_string();
        return Ok(TlsConfig {
            cert_pem,
            key_pem,
            fingerprint,
        });
    }

    let mut params = CertificateParams::new(vec!["localhost".to_string(), "localsend".to_string()])?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "LocalSend Node");
    dn.push(DnType::OrganizationName, "SynapseDrop");
    params.distinguished_name = dn;

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    let cert_der = cert.der();
    let hash = Sha256::digest(cert_der);
    let fingerprint = format!("{:x}", hash);

    let cert_pem = cert.pem().into_bytes();
    let key_pem = key_pair.serialize_pem().into_bytes();

    fs::write(&cert_path, &cert_pem)?;
    fs::write(&key_path, &key_pem)?;
    fs::write(&fingerprint_path, &fingerprint)?;

    Ok(TlsConfig {
        cert_pem,
        key_pem,
        fingerprint,
    })
}
