use std::{path::PathBuf, sync::Arc};

use crate::config::{DomainMatch, ServerConfig};
use anyhow::{Context, Result};
use rustls::{
    crypto::aws_lc_rs::sign::any_supported_type,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    server::{ClientHello, ResolvesServerCert},
    sign::CertifiedKey,
};

#[inline]
pub(crate) fn build_tls_config(config: &Vec<ServerConfig>) -> Result<rustls::ServerConfig> {
    let resolver = TlsResolver::new(config)?;
    Ok(rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(resolver)))
}

#[inline]
fn certified_key(certs: &PathBuf, key: &PathBuf) -> Result<Arc<CertifiedKey>> {
    let certs: Vec<CertificateDer> = CertificateDer::pem_file_iter(certs)
        .context("failed to read tls certificate")?
        .map(|pem| pem.expect("invalid pem"))
        .collect();
    let private_key = PrivateKeyDer::from_pem_file(key).context("invalid private tls key")?;
    Ok(Arc::new(CertifiedKey {
        cert: certs,
        key: any_supported_type(&private_key).context("failed to wrap private key")?,
        ocsp: None,
    }))
}

#[derive(Debug)]
struct TlsEntry {
    domains: Vec<DomainMatch>,
    key: Arc<CertifiedKey>,
}

impl TlsEntry {
    #[inline]
    fn matches(&self, name: &str) -> bool {
        self.domains.is_empty() || self.domains.iter().any(|d| d.0.matches(name))
    }
    #[inline]
    fn key(&self) -> Arc<CertifiedKey> {
        Arc::clone(&self.key)
    }
}

#[derive(Debug)]
pub struct TlsResolver(Vec<TlsEntry>);

impl TlsResolver {
    #[inline]
    pub fn new(config: &Vec<ServerConfig>) -> Result<Self> {
        let mut entries = Vec::new();
        for srv in config.iter() {
            for ssl in srv.listen.iter().filter_map(|l| l.ssl.as_ref()) {
                let key = certified_key(&ssl.certificate, &ssl.certificate_key)?;
                let domains = srv.server_name.clone();
                entries.push(TlsEntry { domains, key })
            }
        }
        Ok(Self(entries))
    }
}

impl ResolvesServerCert for TlsResolver {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        let name = client_hello.server_name().unwrap_or_default();
        self.0
            .iter()
            .find(|entry| entry.matches(name))
            .map(|entry| entry.key())
    }
}
