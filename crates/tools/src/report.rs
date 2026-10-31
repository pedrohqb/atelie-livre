//! Relatório da extração (`report.json`).

use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub processed: Vec<FileSummary>,
    pub failures: Vec<Failure>,
    pub missing: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FileSummary {
    pub path: String,
    pub kind: &'static str,
    pub items_ok: usize,
    pub items_failed: usize,
}

#[derive(Debug, Serialize)]
pub struct Failure {
    pub path: String,
    pub item: String,
    pub error: String,
}

impl Report {
    /// Código de saída: 2 se nada foi reconhecido, 1 se houve falhas, 0 caso contrário.
    pub fn exit_code(&self) -> u8 {
        if self.processed.is_empty() {
            2
        } else if !self.failures.is_empty() {
            1
        } else {
            0
        }
    }
}
