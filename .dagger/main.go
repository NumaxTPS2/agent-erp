package main

import (
	"context"
	"dagger/agent-erp/internal/dagger"
	"fmt"

	"golang.org/x/sync/errgroup"
)

type AgentErp struct{}

// getRustContainer returns a container pre-configured with Rust 1.x toolchain and Linux Tauri system dependencies.
// System dependencies are installed without recommended packages to conserve disk space, with AllowInsecureRepositories flags to avoid GPG errors.
func (m *AgentErp) getRustContainer(src *dagger.Directory) *dagger.Container {
	return dag.Container().
		From("rust:1-bookworm").
		WithEnvVariable("DEBIAN_FRONTEND", "noninteractive").
		WithExec([]string{"sh", "-c", "apt-get update -o Acquire::AllowInsecureRepositories=true -o Acquire::AllowDowngradeToInsecureRepositories=true && apt-get install -y --allow-unauthenticated --no-install-recommends pkg-config build-essential libssl-dev libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev && apt-get clean && rm -rf /var/lib/apt/lists/*"}).
		WithExec([]string{"rustup", "component", "add", "clippy", "rustfmt"}).
		WithMountedCache("/usr/local/cargo/registry", dag.CacheVolume("cargo-registry")).
		WithMountedCache("/usr/local/cargo/git", dag.CacheVolume("cargo-git")).
		WithMountedCache("/src/src-tauri/target", dag.CacheVolume("cargo-target")).
		WithDirectory("/src", src).
		WithWorkdir("/src/src-tauri")
}

// getNodeContainer returns a container pre-configured with Node.js 22 for frontend build.
func (m *AgentErp) getNodeContainer(src *dagger.Directory) *dagger.Container {
	return dag.Container().
		From("node:22-bookworm").
		WithMountedCache("/src/node_modules", dag.CacheVolume("node-modules")).
		WithDirectory("/src", src).
		WithWorkdir("/src")
}

// RunAllChecks runs all required quality gate checks:
// - cargo fmt --check
// - cargo clippy -- -D warnings
// - cargo test
// - npm run build
func (m *AgentErp) RunAllChecks(ctx context.Context, src *dagger.Directory) error {
	g, ctx := errgroup.WithContext(ctx)

	rustBase := m.getRustContainer(src)
	nodeBase := m.getNodeContainer(src)

	// 1. Rust checks (fmt, clippy, test sequentially in rust container)
	safeGo(g, func() error {
		_, err := rustBase.
			WithExec([]string{"cargo", "fmt", "--check"}).
			WithExec([]string{"cargo", "clippy", "--", "-D", "warnings"}).
			WithExec([]string{"cargo", "test"}).
			Sync(ctx)
		if err != nil {
			return fmt.Errorf("rust_checks: execution failed: %w", err)
		}
		return nil
	})

	// 2. Frontend checks (npm ci, npm run build in node container)
	safeGo(g, func() error {
		_, err := nodeBase.
			WithExec([]string{"npm", "ci"}).
			WithExec([]string{"npm", "run", "build"}).
			Sync(ctx)
		if err != nil {
			return fmt.Errorf("npm_build: build failed: %w", err)
		}
		return nil
	})

	return g.Wait()
}

func safeGo(g *errgroup.Group, fn func() error) {
	g.Go(func() (err error) {
		defer func() {
			if r := recover(); r != nil {
				err = fmt.Errorf("panic in goroutine: %v", r)
			}
		}()
		return fn()
	})
}
