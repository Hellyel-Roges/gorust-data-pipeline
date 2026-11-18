package engine

import (
	"crypto/pbkdf2"
	"crypto/sha256"
	"encoding/hex"
	"regexp"
	"sync"
	"time"
)

type CryptoResult struct {
	HashHex   string `json:"hash_hex"`
	ComputeNs uint64 `json:"compute_ns"`
}

type DlpResult struct {
	SanitizedLogs   []string `json:"sanitized_logs"`
	RedactionsCount uint32   `json:"redactions_count"`
	ComputeNs       uint64   `json:"compute_ns"`
}

type ConcurrencyResult struct {
	CompletedTasks uint32 `json:"completed_tasks"`
	Checksum       uint64 `json:"checksum"`
	ComputeNs      uint64 `json:"compute_ns"`
}

var (
	cpfRegex   = regexp.MustCompile(`\b[0-9]{3}\.[0-9]{3}\.[0-9]{3}-[0-9]{2}\b`)
	emailRegex = regexp.MustCompile(`\b[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}\b`)
	tokenRegex = regexp.MustCompile(`\bsk_(?:live|test)_[a-zA-Z0-9]{16,}\b`)
)

func sha256Prefix8(input string) string {
	sum := sha256.Sum256([]byte(input))
	return hex.EncodeToString(sum[:4])
}

// DeriveKey executa PBKDF2-HMAC-SHA256 (32 bytes em hex) usando a stdlib do Go.
func DeriveKey(payload, salt []byte, iterations uint32) (CryptoResult, error) {
	start := time.Now()
	rounds := iterations
	if rounds == 0 {
		rounds = 1
	}

	dk, err := pbkdf2.Key(sha256.New, string(payload), salt, int(rounds), 32)
	if err != nil {
		return CryptoResult{}, err
	}

	return CryptoResult{
		HashHex:   hex.EncodeToString(dk),
		ComputeNs: uint64(time.Since(start).Nanoseconds()),
	}, nil
}

// SanitizeLogs aplica detecção RE2 + pseudonimização SHA-256 em lote.
func SanitizeLogs(logs []string) DlpResult {
	start := time.Now()
	var redactions uint32
	sanitized := make([]string, len(logs))

	for i, line := range logs {
		s := cpfRegex.ReplaceAllStringFunc(line, func(m string) string {
			redactions++
			return "[CPF:" + sha256Prefix8(m) + "]"
		})
		s = emailRegex.ReplaceAllStringFunc(s, func(m string) string {
			redactions++
			return "[EMAIL:" + sha256Prefix8(m) + "]"
		})
		s = tokenRegex.ReplaceAllStringFunc(s, func(m string) string {
			redactions++
			return "[TOKEN:" + sha256Prefix8(m) + "]"
		})
		sanitized[i] = s
	}

	return DlpResult{
		SanitizedLogs:   sanitized,
		RedactionsCount: redactions,
		ComputeNs:       uint64(time.Since(start).Nanoseconds()),
	}
}

// RunConcurrency executa fan-out / fan-in concorrente via Goroutines e channel bufferizado.
func RunConcurrency(tasks, ioDelayMs uint32) ConcurrencyResult {
	start := time.Now()
	if tasks == 0 {
		return ConcurrencyResult{
			CompletedTasks: 0,
			Checksum:       0,
			ComputeNs:      uint64(time.Since(start).Nanoseconds()),
		}
	}

	bufSize := int(tasks)
	if bufSize > 1024 {
		bufSize = 1024
	}
	ch := make(chan uint64, bufSize)

	var wg sync.WaitGroup
	wg.Add(int(tasks))
	delay := time.Duration(ioDelayMs) * time.Millisecond

	for i := uint32(0); i < tasks; i++ {
		go func(idx uint64) {
			defer wg.Done()
			if delay > 0 {
				time.Sleep(delay)
			}
			val := (idx * 2654435761) ^ 0x9e3779b97f4a7c15
			ch <- val
		}(uint64(i))
	}

	go func() {
		wg.Wait()
		close(ch)
	}()

	var checksum uint64
	var completed uint32
	for val := range ch {
		checksum += val
		completed++
	}

	return ConcurrencyResult{
		CompletedTasks: completed,
		Checksum:       checksum,
		ComputeNs:      uint64(time.Since(start).Nanoseconds()),
	}
}
