# GoRust Data Pipeline — Hybrid Architecture & Performance Arena

Laboratório de arquitetura de software e análise de performance projetado para medir, sob cargas controladas e paridade algorítmica bit-a-bit, os *trade-offs* entre **Go** e **Rust** em arquiteturas puras e híbridas.

O projeto responde a duas perguntas práticas de engenharia de sistemas:
1. **Qual é o ponto de inflexão real para delegar computação pesada de um Gateway Go para o Rust?** (CGO/FFI em memória vs. microsserviço gRPC em rede).
2. **Como se comportam Go (`net/http` + Goroutines) e Rust (`Axum` + `Tokio`) lado a lado** sob pressão de CPU, alocação de memória (*GC jitter*) e concorrência massiva?

---

## Matriz de Experimentos (4 Topologias x 3 Workloads)

Todas as implementações em Rust compartilham exatamente o mesmo núcleo lógico (`rust-core`) via **Cargo Workspace**, enquanto o motor em Go puro (`api-go/internal/engine`) espelha os mesmos algoritmos com saídas validadas por testes unitários cruzados. Cada resposta retorna `compute_ns` para isolar o **tempo puro de computação** do **overhead de transporte, serialização ou FFI**.

### Topologias Avaliadas

1. **Go Puro (`api-go`):** Servidor HTTP nativo (`net/http`) + processamento via Goroutines e stdlib (baseline).
2. **Rust Puro (`api-rust`):** Servidor HTTP assíncrono (`Axum` + `Tokio`) consumindo `rust-core` diretamente.
3. **Go + Rust via FFI / CGO (`api-go` -> `rust-ffi`):** Gateway Go invocando uma biblioteca compartilhada C (`.so`) em Rust, eliminando latência de rede ao custo da fronteira CGO e gestão explícita de memória.
4. **Go + Rust via gRPC (`api-go` -> `rust-grpc`):** Gateway Go roteando chamadas via HTTP/2 + Protocol Buffers para um worker independente em Rust (`Tonic` + `Tokio`), priorizando isolamento e escalabilidade horizontal.

### Cargas de Trabalho (Workloads)

| Workload | Algoritmo | O que estressa e evidencia |
| :--- | :--- | :--- |
| **`/crypto`** | PBKDF2-HMAC-SHA256 com `iterations` configurável | Operação **CPU-bound pura** com payload pequeno; permite mapear a curva exata onde o ganho de CPU do Rust supera o overhead de CGO e de rede do gRPC. |
| **`/dlp`** | Varredura de logs em lote via Regex RE2 (CPF, e-mail, tokens `sk_live_`/`sk_test_`) + pseudonimização SHA-256 | Pressão de alocação de strings, cópia na fronteira FFI/Protobuf e impacto de **GC jitter** no Go frente ao modelo de *ownership* do Rust. |
| **`/concurrency`** | *Fan-out / Fan-in* de $N$ tarefas simultâneas com simulação de I/O e agregação determinística | Escalonador M:N do Go (Goroutines + channels) vs. *work-stealing* do Tokio (tasks + `mpsc`), foco em consumo de RAM e cauda de latência (**P95/P99**). |

---

## Arquitetura do Repositório

```text
gorust-data-pipeline/
├── Cargo.toml                  # Cargo Workspace unificando as crates Rust
├── docker-compose.yml          # Orquestração com limites rígidos de CPU/RAM por container
├── proto/
│   └── pipeline.proto          # Contrato gRPC único (Crypto, DLP, Concurrency)
├── rust-core/                  # Núcleo algorítmico compartilhado em Rust
├── rust-ffi/                   # Adaptador C ABI (cdylib) consumido via CGO
├── rust-grpc/                  # Worker gRPC assíncrono (Tonic + Tokio)
├── api-rust/                   # Servidor HTTP em Rust puro (Axum + Tokio)
├── api-go/                     # Gateway HTTP Go (/go/*, /cgo/*, /grpc/*)
│   └── internal/engine/        # Motor espelho em Go puro + testes de paridade
├── benchmark/
│   ├── main.go                 # Load generator (RPS, P50/P90/P95/P99, CPU/RAM)
│   └── plot.py                 # Renderização automática dos gráficos a partir do raw JSON
└── results/
    ├── raw/                    # Artefatos brutos das execuções (JSON/CSV)
    └── charts/                 # Gráficos versionados (.png) para análise

```

---

## Stack Tecnológico

* **Linguagens:** Go 1.27+, Rust (Edition 2021 / Cargo Workspace)
* **HTTP & Async:** Go `net/http` / Goroutines, Rust `Axum` / `Tokio`
* **Integração & IPC:** CGO / FFI (`cdylib`, C ABI), gRPC (`Tonic` / `google.golang.org/grpc`), Protocol Buffers (`proto3`)
* **Infraestrutura & Isolamento:** Docker, Docker Compose (limites fixos de vCPU e RAM)
* **Análise & Visualização:** Go load harness + Python (`pandas`, `matplotlib`)

---

## Validação de Paridade (Testes Atuais)

Os motores `rust-core` e `api-go/internal/engine` são validados contra os mesmos vetores determinísticos (incluindo RFC 6070 para PBKDF2):

```bash
# Testes unitários e lint estrito no workspace Rust
cargo test -p rust-core
cargo clippy --workspace --all-targets -- -D warnings

# Testes unitários com detector de data races no motor Go
(cd api-go && go test -v -race ./internal/engine/...)

```

---

## Metodologia de Benchmark e Resultados

Para garantir rigor estatístico e reprodutibilidade:

* **Isolamento de recursos:** todos os serviços executam em containers com limites idênticos de CPU e memória, compilados em modo `--release` (Rust) e binário otimizado (Go).
* **Warm-up:** os primeiros segundos de carga são descartados antes da janela de medição para estabilização de pools HTTP/2, caches e alocadores.
* **Métricas coletadas:** Throughput (RPS), latência de ponta a ponta e de cauda (**P50, P95, P99**), `compute_ns` médio, taxa de erro e pico de memória **RSS** / **CPU%**.

Os dados brutos serão salvos em `results/raw/` e os gráficos gerados automaticamente em `results/charts/` após a conclusão da suíte de benchmark.
