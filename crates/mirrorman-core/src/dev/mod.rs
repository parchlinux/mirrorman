pub mod detectors;
pub mod mirrors;
pub mod models;
pub mod tester;

pub use detectors::EcosystemDetector;
pub use models::{DevEcosystemKind, DevMirror, EcosystemStatus};

use detectors::{
    cargo::CargoDetector, composer::ComposerDetector, gem::GemDetector, go::GoDetector,
    npm::NpmDetector, pip::PipDetector,
};

pub fn get_detector(kind: DevEcosystemKind) -> Box<dyn EcosystemDetector> {
    match kind {
        DevEcosystemKind::Pip => Box::new(PipDetector::new()),
        DevEcosystemKind::Npm => Box::new(NpmDetector::new()),
        DevEcosystemKind::Cargo => Box::new(CargoDetector::new()),
        DevEcosystemKind::Go => Box::new(GoDetector::new()),
        DevEcosystemKind::Gem => Box::new(GemDetector::new()),
        DevEcosystemKind::Composer => Box::new(ComposerDetector::new()),
    }
}

pub fn scan_ecosystems() -> Vec<EcosystemStatus> {
    DevEcosystemKind::ALL
        .iter()
        .map(|&kind| {
            let detector = get_detector(kind);
            let candidate_mirrors = mirrors::get_mirrors_for_ecosystem(kind);
            detector.get_status(candidate_mirrors)
        })
        .collect()
}

pub fn test_ecosystem_mirrors(kind: DevEcosystemKind, workers: usize) -> Vec<DevMirror> {
    let mut candidate_mirrors = mirrors::get_mirrors_for_ecosystem(kind);
    tester::test_dev_mirrors(&mut candidate_mirrors, workers);
    candidate_mirrors
}

pub fn set_ecosystem_mirror(kind: DevEcosystemKind, mirror_name_or_url: &str) -> Result<DevMirror, String> {
    let detector = get_detector(kind);
    let candidate_mirrors = mirrors::get_mirrors_for_ecosystem(kind);

    let found = candidate_mirrors
        .into_iter()
        .find(|m| {
            m.name.eq_ignore_ascii_case(mirror_name_or_url)
                || m.url.trim_end_matches('/') == mirror_name_or_url.trim_end_matches('/')
        })
        .unwrap_or_else(|| {
            // If user supplied a custom URL not in default list
            DevMirror {
                name: mirror_name_or_url.to_string(),
                url: mirror_name_or_url.to_string(),
                country: "Custom".to_string(),
                country_code: "XX".to_string(),
                official: false,
                ping_url: mirror_name_or_url.to_string(),
                trusted_host: None,
                speed: None,
            }
        });

    detector.set_mirror(&found)?;
    Ok(found)
}

pub fn reset_ecosystem_mirror(kind: DevEcosystemKind) -> Result<(), String> {
    let detector = get_detector(kind);
    detector.reset_mirror()
}

pub fn auto_select_fastest(workers: usize) -> Vec<(DevEcosystemKind, Result<DevMirror, String>)> {
    let mut results = Vec::new();
    for &kind in &DevEcosystemKind::ALL {
        let detector = get_detector(kind);
        if !detector.is_installed() {
            continue;
        }

        let tested = test_ecosystem_mirrors(kind, workers);
        if let Some(fastest) = tested.into_iter().find(|m| m.speed.is_some()) {
            let res = detector.set_mirror(&fastest).map(|_| fastest);
            results.push((kind, res));
        } else {
            results.push((kind, Err("No responsive mirrors found".to_string())));
        }
    }
    results
}
