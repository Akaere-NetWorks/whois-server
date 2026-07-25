\
# Makefile — convenience wrapper around the dotnet CLI for local dev.
# Run `make` or `make help` to list targets.

SOLUTION       := whois-server.slnx
HOST_PROJECT   := src/WhoisServer.Host/WhoisServer.Host.csproj
CORE_TESTS     := tests/WhoisServer.Core.Tests/WhoisServer.Core.Tests.csproj
INTEGRATION_TESTS := tests/WhoisServer.Integration.Tests/WhoisServer.Integration.Tests.csproj
CONFIGURATION  ?= Debug
PORT           ?= 4343
ARGS           ?=
FEATURES       ?=

RID            ?= linux-x64

.PHONY: help restore build build-release watch run test test-unit test-integration \
        format format-check clean minimal publish deb rpm packages

help: ## Show this help
	@echo "Targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Variables: CONFIGURATION=$(CONFIGURATION) PORT=$(PORT) RID=$(RID) FEATURES='$(FEATURES)'"
	@echo "Examples:"
	@echo "  make run ARGS='--debug'"
	@echo "  make build FEATURES='-p:Feature_Dn42=false'   # real cut build, see plan.md §6"
	@echo "  make packages RID=linux-arm64 VERSION=1.4.0   # .deb + .rpm"

restore: ## Restore NuGet packages for the whole solution
	dotnet restore $(SOLUTION)

build: ## Build the whole solution (Debug by default; FEATURES=... for cut builds)
	dotnet build $(SOLUTION) -c $(CONFIGURATION) $(FEATURES)

build-release: ## Build the whole solution in Release configuration
	$(MAKE) build CONFIGURATION=Release

watch: ## Run the WHOIS host under `dotnet watch` (auto-restart on file changes)
	dotnet watch --project $(HOST_PROJECT) run -- --port $(PORT) --debug $(ARGS)

run: ## Run the WHOIS host (defaults to port $(PORT) so it doesn't need root; pass ARGS='...' for extra flags)
	dotnet run --project $(HOST_PROJECT) -c $(CONFIGURATION) -- --port $(PORT) $(ARGS)

test: ## Run all tests (unit + live-network integration tests)
	dotnet test $(SOLUTION) -c $(CONFIGURATION)

test-unit: ## Run only WhoisServer.Core.Tests (no network access needed)
	dotnet test $(CORE_TESTS) -c $(CONFIGURATION)

test-integration: ## Run only WhoisServer.Integration.Tests (hits real WHOIS/RDAP/HTTP infra)
	dotnet test $(INTEGRATION_TESTS) -c $(CONFIGURATION)

format: ## Apply dotnet-format code style fixes
	dotnet format $(SOLUTION)

format-check: ## Check code style without modifying files (CI-friendly)
	dotnet format $(SOLUTION) --verify-no-changes

minimal: ## Build with DN42 excluded, proving the real cut-build mechanism (plan.md §6)
	$(MAKE) build FEATURES='-p:Feature_Dn42=false'

publish: ## Publish a self-contained release build of the WHOIS host
	dotnet publish $(HOST_PROJECT) -c Release -o publish

deb: ## Build a .deb into dist/ (RID=linux-x64|linux-arm64, VERSION=... overrides Directory.Build.props)
	packaging/deb/build-deb.sh --rid $(RID) \
		$(if $(VERSION),--version $(VERSION),) \
		$(if $(FEATURES),--features '$(FEATURES)',)

rpm: ## Build an .rpm into dist/ (same RID/VERSION/FEATURES variables as `make deb`)
	packaging/rpm/build-rpm.sh --rid $(RID) \
		$(if $(VERSION),--version $(VERSION),) \
		$(if $(FEATURES),--features '$(FEATURES)',)

packages: deb rpm ## Build both the .deb and the .rpm for $(RID)

clean: ## Remove build artifacts (bin/obj) and the local dev cache/ directory
	dotnet clean $(SOLUTION)
	find src tests -type d \( -name bin -o -name obj \) -prune -exec rm -rf {} +
	rm -rf cache publish dist
