use pbkdf2::pbkdf2_hmac;
use regex::{Captures, Regex};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoResult {
    pub hash_hex: String,
    pub compute_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DlpResult {
    pub sanitized_logs: Vec<String>,
    pub redactions_count: u32,
    pub compute_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrencyResult {
    pub completed_tasks: u32,
    pub checksum: u64,
    pub compute_ns: u64,
}

struct DlpRules {
    cpf: Regex,
    email: Regex,
    token: Regex,
}

fn dlp_rules() -> &'static DlpRules {
    static RULES: OnceLock<DlpRules> = OnceLock::new();
    RULES.get_or_init(|| DlpRules {
        cpf: Regex::new(r"\b[0-9]{3}\.[0-9]{3}\.[0-9]{3}-[0-9]{2}\b")
            .expect("valid cpf regex"),
        email: Regex::new(r"\b[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}\b")
            .expect("valid email regex"),
        token: Regex::new(r"\bsk_(?:live|test)_[a-zA-Z0-9]{16,}\b")
            .expect("valid token regex"),
    })
}

fn sha256_prefix_8(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex::encode(&digest[..4])
}

/// Workload 1 (CPU-bound): PBKDF2-HMAC-SHA256 determinístico (32 bytes em hex).
pub fn derive_key(payload: &[u8], salt: &[u8], iterations: u32) -> CryptoResult {
    let start = Instant::now();
    let rounds = iterations.max(1);
    let mut derived_key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(payload, salt, rounds, &mut derived_key);

    CryptoResult {
        hash_hex: hex::encode(derived_key),
        compute_ns: start.elapsed().as_nanos() as u64,
    }
}

/// Workload 2 (DLP / Memória): Detecção via Regex RE2 + pseudonimização SHA-256.
pub fn sanitize_logs(logs: &[String]) -> DlpResult {
    let start = Instant::now();
    let rules = dlp_rules();
    let mut redactions_count: u32 = 0;
    let mut sanitized_logs = Vec::with_capacity(logs.len());

    for log in logs {
        let step1 = rules.cpf.replace_all(log, |caps: &Captures<'_>| {
            redactions_count += 1;
            format!("[CPF:{}]", sha256_prefix_8(&caps[0]))
        });
        let step2 = rules.email.replace_all(&step1, |caps: &Captures<'_>| {
            redactions_count += 1;
            format!("[EMAIL:{}]", sha256_prefix_8(&caps[0]))
        });
        let step3 = rules.token.replace_all(&step2, |caps: &Captures<'_>| {
            redactions_count += 1;
            format!("[TOKEN:{}]", sha256_prefix_8(&caps[0]))
        });
        sanitized_logs.push(step3.into_owned());
    }

    DlpResult {
        sanitized_logs,
        redactions_count,
        compute_ns: start.elapsed().as_nanos() as u64,
    }
}

/// Workload 3 (Concorrência Async): Fan-out / Fan-in via Tokio tasks + mpsc channel.
pub async fn run_concurrency_async(tasks: u32, io_delay_ms: u32) -> ConcurrencyResult {
    let start = Instant::now();
    if tasks == 0 {
        return ConcurrencyResult {
            completed_tasks: 0,
            checksum: 0,
            compute_ns: start.elapsed().as_nanos() as u64,
        };
    }

    let buffer = (tasks as usize).clamp(1, 1024);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<u64>(buffer);

    for i in 0..tasks {
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            if io_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(u64::from(io_delay_ms))).await;
            }
            let val = u64::from(i).wrapping_mul(2_654_435_761) ^ 0x9e37_79b9_7f4a_7c15;
            let _ = tx_clone.send(val).await;
        });
    }
    drop(tx);

    let mut checksum: u64 = 0;
    let mut completed_tasks: u32 = 0;
    while let Some(val) = rx.recv().await {
        checksum = checksum.wrapping_add(val);
        completed_tasks += 1;
    }

    ConcurrencyResult {
        completed_tasks,
        checksum,
        compute_ns: start.elapsed().as_nanos() as u64,
    }
}

fn shared_runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .build()
            .expect("failed to build shared tokio runtime")
    })
}

/// Bridge síncrona reutilizando o pool Tokio global (consumida pelo rust-ffi via CGO).
pub fn run_concurrency_blocking(tasks: u32, io_delay_ms: u32) -> ConcurrencyResult {
    shared_runtime().block_on(run_concurrency_async(tasks, io_delay_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_parity_vectors() {
        let rfc = derive_key(b"password", b"salt", 1);
        assert_eq!(
            rfc.hash_hex,
            "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"
        );

        let workload = derive_key(b"gorust-pipeline-payload", b"static-salt-v1", 1000);
        assert_eq!(
            workload.hash_hex,
            "f4152bcadb90c53508509eb1064099160f11193b7a79421b5e0d5d342e1212ca"
        );
    }

    #[test]
    fn test_sanitize_logs_parity_vector() {
        let input = vec![
            "user cpf=123.456.789-00 email=hellyel@example.com auth=sk_live_9f8e7d6c5b4a32109876".to_string(),
            "clean log line without sensitive data".to_string(),
        ];

        let res = sanitize_logs(&input);
        assert_eq!(res.redactions_count, 3);
        assert_eq!(
            res.sanitized_logs[0],
            "user cpf=[CPF:f788964b] email=[EMAIL:b992fd87] auth=[TOKEN:eb35dec5]"
        );
        assert_eq!(res.sanitized_logs[1], input[1]);
    }

    #[tokio::test]
    async fn test_concurrency_parity_vector() {
        let res = run_concurrency_async(1000, 1).await;
        assert_eq!(res.completed_tasks, 1000);
        assert_eq!(res.checksum, 626_701_647_711_576_428);
    }
}