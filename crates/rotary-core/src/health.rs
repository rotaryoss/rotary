use serde::{Deserialize, Serialize};

/// Severity level for a health check finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Ok,
    Warning,
    Critical,
}

/// A single health check finding for one secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub key: String,
    pub severity: Severity,
    pub reason: String,
}

/// Overall health score (0–100).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HealthScore(pub u8);

impl HealthScore {
    pub fn compute(checks: &[HealthCheck]) -> Self {
        if checks.is_empty() {
            return Self(100);
        }

        let total = checks.len() as f64;
        let deductions: f64 = checks
            .iter()
            .map(|c| match c.severity {
                Severity::Critical => 1.0,
                Severity::Warning => 0.5,
                Severity::Ok => 0.0,
            })
            .sum();

        let score = ((1.0 - deductions / total) * 100.0).round().max(0.0) as u8;
        Self(score)
    }
}

/// The full scan report returned by the scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub source_name: String,
    pub environment: String,
    pub total_secrets: usize,
    pub checks: Vec<HealthCheck>,
    pub score: HealthScore,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_all_ok() {
        let checks = vec![
            HealthCheck { key: "A".into(), severity: Severity::Ok, reason: String::new() },
            HealthCheck { key: "B".into(), severity: Severity::Ok, reason: String::new() },
        ];
        assert_eq!(HealthScore::compute(&checks).0, 100);
    }

    #[test]
    fn score_mixed() {
        let checks = vec![
            HealthCheck { key: "A".into(), severity: Severity::Critical, reason: String::new() },
            HealthCheck { key: "B".into(), severity: Severity::Warning, reason: String::new() },
            HealthCheck { key: "C".into(), severity: Severity::Ok, reason: String::new() },
            HealthCheck { key: "D".into(), severity: Severity::Ok, reason: String::new() },
        ];
        // deductions: 1.0 + 0.5 + 0 + 0 = 1.5 out of 4 → (1 - 0.375) * 100 = 62.5 → 63
        assert_eq!(HealthScore::compute(&checks).0, 63);
    }

    #[test]
    fn score_all_critical() {
        let checks = vec![
            HealthCheck { key: "A".into(), severity: Severity::Critical, reason: String::new() },
            HealthCheck { key: "B".into(), severity: Severity::Critical, reason: String::new() },
        ];
        assert_eq!(HealthScore::compute(&checks).0, 0);
    }

    #[test]
    fn score_empty() {
        assert_eq!(HealthScore::compute(&[]).0, 100);
    }
}
