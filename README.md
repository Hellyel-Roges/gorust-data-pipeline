# GoRust Hybrid Architecture & Performance Arena

Este repositório é um laboratório de arquitetura de software desenhado para explorar os *trade-offs* de integração entre **Go** e **Rust** em cenários de alta demanda (CPU-bound e I/O-bound).

O objetivo é responder a uma pergunta prática de engenharia: *"Qual é a forma mais eficiente de delegar processamento pesado para o Rust a partir de um Gateway em Go?"*

Para isso, o ecossistema compara três abordagens sob testes de carga (Load Testing):
1. **Go Puro:** Processamento nativo via Goroutines (Baseline).
2. **Integração FFI / CGO:** Go chamando uma biblioteca dinâmica (`.so`) escrita em Rust, focando em zero latência de rede.
3. **Microsserviços gRPC:** Go atuando como API Gateway roteando dados pesados via Protobuf para um serviço Rust isolado, focando em escalabilidade independente.

## Stack Tecnológico
* **Linguagens:** Go, Rust
* **Comunicação & Integração:** gRPC, Protocol Buffers, CGO / FFI
* **Infraestrutura:** Docker, Docker Compose
