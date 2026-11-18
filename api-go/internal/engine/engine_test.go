package engine

import "testing"

func TestDeriveKeyParityVectors(t *testing.T) {
	rfc, err := DeriveKey([]byte("password"), []byte("salt"), 1)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	const wantRFC = "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"
	if rfc.HashHex != wantRFC {
		t.Fatalf("RFC mismatch: got %s, want %s", rfc.HashHex, wantRFC)
	}

	workload, err := DeriveKey([]byte("gorust-pipeline-payload"), []byte("static-salt-v1"), 1000)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	const wantWorkload = "f4152bcadb90c53508509eb1064099160f11193b7a79421b5e0d5d342e1212ca"
	if workload.HashHex != wantWorkload {
		t.Fatalf("workload mismatch: got %s, want %s", workload.HashHex, wantWorkload)
	}
}

func TestSanitizeLogsParityVector(t *testing.T) {
	input := []string{
		"user cpf=123.456.789-00 email=hellyel@example.com auth=sk_live_9f8e7d6c5b4a32109876",
		"clean log line without sensitive data",
	}

	res := SanitizeLogs(input)
	if res.RedactionsCount != 3 {
		t.Fatalf("redactions mismatch: got %d, want 3", res.RedactionsCount)
	}

	const wantLine0 = "user cpf=[CPF:f788964b] email=[EMAIL:b992fd87] auth=[TOKEN:eb35dec5]"
	if res.SanitizedLogs[0] != wantLine0 {
		t.Fatalf("line 0 mismatch: got %q, want %q", res.SanitizedLogs[0], wantLine0)
	}
	if res.SanitizedLogs[1] != input[1] {
		t.Fatalf("line 1 altered: got %q, want %q", res.SanitizedLogs[1], input[1])
	}
}

func TestRunConcurrencyParityVector(t *testing.T) {
	res := RunConcurrency(1000, 1)
	if res.CompletedTasks != 1000 {
		t.Fatalf("completed mismatch: got %d, want 1000", res.CompletedTasks)
	}
	const wantChecksum uint64 = 626701647711576428
	if res.Checksum != wantChecksum {
		t.Fatalf("checksum mismatch: got %d, want %d", res.Checksum, wantChecksum)
	}
}
