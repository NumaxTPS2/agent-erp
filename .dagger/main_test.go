package main

import (
	"errors"
	"testing"

	"golang.org/x/sync/errgroup"
)

func TestSafeGo(t *testing.T) {
	tests := []struct {
		name        string
		fn          func() error
		expectError bool
		errMessage  string
	}{
		{
			name: "successful execution",
			fn: func() error {
				return nil
			},
			expectError: false,
		},
		{
			name: "returns error",
			fn: func() error {
				return errors.New("sample error")
			},
			expectError: true,
			errMessage:  "sample error",
		},
		{
			name: "recovers panic and returns error",
			fn: func() error {
				panic("something went wrong")
			},
			expectError: true,
			errMessage:  "panic in goroutine: something went wrong",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var g errgroup.Group
			safeGo(&g, tt.fn)
			err := g.Wait()

			if tt.expectError {
				if err == nil {
					t.Fatalf("expected error containing %q, got nil", tt.errMessage)
				}
				if tt.errMessage != "" && err.Error() != tt.errMessage {
					t.Errorf("expected error %q, got %q", tt.errMessage, err.Error())
				}
			} else {
				if err != nil {
					t.Fatalf("expected no error, got %v", err)
				}
			}
		})
	}
}
