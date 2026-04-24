//! Background operations — spec loading and action execution.

use super::types::DomainId;
use crate::registry::Registry;
use crate::spec::DomainSpec;
use std::sync::{Arc, mpsc};

/// Start a background spec load for a single domain.
pub fn spawn_spec_load(
    id: DomainId,
    domain_name: &str,
    reg: &Arc<dyn Registry>,
) -> mpsc::Receiver<(DomainId, Option<DomainSpec>)> {
    let (tx, rx) = mpsc::channel();
    let domain = domain_name.to_string();
    let reg = Arc::clone(reg);
    std::thread::spawn(move || {
        let spec = reg.fetch_spec(&domain);
        let _ = tx.send((id, spec));
    });
    rx
}

/// Start background preloading of all domain specs.
pub fn spawn_preload_all(
    domains: &[(DomainId, String)],
    reg: &Arc<dyn Registry>,
) -> mpsc::Receiver<(DomainId, Option<DomainSpec>)> {
    let (tx, rx) = mpsc::channel();
    let domains: Vec<_> = domains.to_vec();
    let reg = Arc::clone(reg);
    std::thread::spawn(move || {
        for (id, domain) in domains {
            let spec = reg.fetch_spec(&domain);
            let _ = tx.send((id, spec));
        }
    });
    rx
}

/// Start a background action execution.
pub fn spawn_action(
    id: DomainId,
    domain: &str,
    command: &str,
    args: &[String],
    reload: bool,
    reg: &Arc<dyn Registry>,
) -> mpsc::Receiver<(DomainId, bool, String)> {
    let (tx, rx) = mpsc::channel();
    let domain = domain.to_string();
    let command = command.to_string();
    let args = args.to_vec();
    let reg = Arc::clone(reg);
    std::thread::spawn(move || {
        let result = reg.run_action(&domain, &command, &args);
        let _ = tx.send((id, reload, result));
    });
    rx
}
