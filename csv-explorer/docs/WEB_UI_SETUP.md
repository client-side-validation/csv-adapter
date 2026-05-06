# CSV Explorer Web UI - Setup Guide

## Current Status

✅ **API Server**: Fully functional on port 8080  
⚠️ **Web UI**: Requires WASM compilation

## Why the Web UI Isn't Working

The CSV Explorer UI is built with Dioxus, which compiles to WebAssembly (WASM) for web deployment. The current binary (`csv-explorer-ui`) was compiled as a standard Rust binary, not as a WASM web app.

To serve the Web UI, you need to compile the UI to WASM first.

## Solution: Build and Serve Web UI

### Option 1: Use Dioxus CLI (Recommended)

**Step 1: Install Dioxus CLI**
```bash
cargo install dioxus-cli
```

**Step 2: Serve with Hot Reload**
```bash
cd /home/zorvan/Work/projects/csv-adapter/csv-explorer/ui
dx serve
```

This will:
- Compile the UI to WASM
- Start a development server on port 8080
- Enable hot reload for development

**Step 3: Open Browser**
```
http://localhost:8080
```

### Option 2: Build WASM and Serve Statically

**Step 1: Install Dioxus CLI**
```bash
cargo install dioxus-cli
```

**Step 2: Build WASM**
```bash
cd /home/zorvan/Work/projects/csv-adapter/csv-explorer/ui
dx build --platform web --release
```

This creates a `dist` directory with:
- `index.html`
- WASM binary
- JavaScript loader
- Assets (CSS, etc.)

**Step 3: Serve the Built Files**

Use any static file server:

**Using Python:**
```bash
cd ui/dist
python3 -m http.server 3000
```

**Using Node.js:**
```bash
npx serve ui/dist -p 3000
```

**Using Caddy:**
```bash
cd ui/dist
caddy file-server --listen :3000
```

**Step 4: Open Browser**
```
http://localhost:3000
```

## Alternative: Use API Directly

While the Web UI is being set up, you can use the fully functional API:

### REST API
```bash
# Statistics
curl http://localhost:8080/api/v1/stats

# List Sanads
curl http://localhost:8080/api/v1/sanads?limit=5

# List Transfers
curl http://localhost:8080/api/v1/transfers?limit=5

# List Seals
curl http://localhost:8080/api/v1/seals?limit=5

# List Contracts
curl http://localhost:8080/api/v1/contracts?limit=5

# Chain Status
curl http://localhost:8080/api/v1/chains
```

### GraphQL API
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ stats { totalSanads totalTransfers totalSeals totalContracts } }"}'
```

### Test Data Available

The database is seeded with:
- 8 Sanads across 5 chains
- 7 Cross-chain transfers
- 11 Seals (all types)
- 11 Deployed contracts

## Architecture

```
┌─────────────┐         ┌──────────────┐
│   Browser   │  HTTP   │  Dioxus Web  │
│  (Web UI)   │────────>│  (WASM App)  │
└─────────────┘         └──────┬───────┘
                               │
                        API Calls
                               │
                               v
                        ┌──────────────┐
                        │  API Server  │
                        │  (Port 8080) │
                        └──────┬───────┘
                               │
                        Database
                               │
                               v
                        ┌──────────────┐
                        │  SQLite DB   │
                        │  (explorer.db)│
                        └──────────────┘
```

## Quick Commands

```bash
# Start API server
./target/release/csv-explorer-api start

# Start Web UI (after installing dioxus-cli)
cd ui && dx serve

# Run test suite
./test-api.sh

# Check API health
curl http://localhost:8080/health
```

## Troubleshooting

### "dx: command not found"
```bash
cargo install dioxus-cli
```

### WASM Build Fails
```bash
# Install WASM target
rustup target add wasm32-unknown-unknown

# Clean and rebuild
dx build --platform web --release
```

### Port Already in Use
```bash
# Find process
lsof -i :8080

# Kill it
kill $(lsof -t -i:8080)
```

### API Not Responding
```bash
# Check if running
curl http://localhost:8080/health

# Restart
pkill -f csv-explorer-api
./target/release/csv-explorer-api start
```

## Next Steps

1. **Install dioxus-cli**: `cargo install dioxus-cli`
2. **Build WASM**: `cd ui && dx build --release`
3. **Serve**: Use any static server on port 3000
4. **Access**: Open http://localhost:3000 in browser

## Current Working Features

✅ API Server (port 8080)
  - REST endpoints
  - GraphQL endpoint
  - Health check
  - Statistics
  - All CRUD operations

✅ Database (seeded with test data)
  - 8 sanads
  - 7 transfers
  - 11 seals
  - 11 contracts

⚠️ Web UI (requires WASM build)
  - All components built and tested
  - Routing configured
  - API integration ready
  - Just needs WASM compilation
