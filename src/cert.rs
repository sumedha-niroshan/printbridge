use anyhow::{Context, Result};
use rcgen::{CertificateParams, DistinguishedName, DnType, SanType, KeyPair};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub struct CertPaths {
    pub cert_pem: PathBuf,
    pub key_pem: PathBuf,
}

/// Ensure a self-signed TLS cert exists for localhost.
/// Generates a new one if missing or expired.
pub fn ensure_cert(data_dir: &Path) -> Result<CertPaths> {
    let cert_dir = data_dir.join("certs");
    fs::create_dir_all(&cert_dir).context("Failed to create certs directory")?;

    let cert_pem = cert_dir.join("printbridge.crt");
    let key_pem = cert_dir.join("printbridge.key");

    if cert_pem.exists() && key_pem.exists() {
        if !is_cert_expired(&cert_pem) {
            info!("Using existing TLS certificate at {:?}", cert_pem);
            return Ok(CertPaths { cert_pem, key_pem });
        }
        warn!("TLS certificate expired — regenerating");
    }

    info!("Generating new self-signed TLS certificate...");
    generate_cert(&cert_pem, &key_pem)?;
    info!("Certificate generated at {:?}", cert_pem);

    Ok(CertPaths { cert_pem, key_pem })
}

fn generate_cert(cert_pem: &Path, key_pem: &Path) -> Result<()> {
    let mut params = CertificateParams::default();

    // Valid for 3 years
    params.not_before = rcgen::date_time_ymd(2024, 1, 1);
    params.not_after  = rcgen::date_time_ymd(2027, 1, 1);

    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "PrintBridge Local");
    dn.push(DnType::OrganizationName, "PrintBridge");
    params.distinguished_name = dn;

    params.subject_alt_names = vec![
        SanType::DnsName("localhost".to_string()),
        SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
    ];

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    fs::write(cert_pem, cert.pem()).context("Failed to write cert PEM")?;
    fs::write(key_pem, key_pair.serialize_pem()).context("Failed to write key PEM")?;

    Ok(())
}

fn is_cert_expired(cert_pem: &Path) -> bool {
    // Simple heuristic: if file is older than 2.5 years (in seconds), regenerate
    if let Ok(meta) = fs::metadata(cert_pem) {
        if let Ok(modified) = meta.modified() {
            if let Ok(age) = modified.elapsed() {
                // 2.5 years ≈ 78840000 seconds
                return age.as_secs() > 78_840_000;
            }
        }
    }
    false
}
