# CSV Explorer - Dioxus Web UI Guide

## Quick Start

The CSV Explorer UI is a Dioxus web application that connects to the API server to display sanads, transfers, seals, contracts, and statistics.

### Step 1: Start the API Server

First, ensure the API server is running:

```bash
# Start API server
./target/release/csv-explorer-api start

# Verify it's running
curl http://localhost:8080/health
# Should return: {"service":"csv-explorer-api","status":"ok"}
```

### Step 2: Start the Web UI

The UI server connects to the API and serves the web interface:

```bash
# Start UI server (default: port 3000)
./target/release/csv-explorer-ui serve
```

You'll see:
```
Starting CSV Explorer UI in web mode...
Open http://localhost:3000 in your browser
```

### Step 3: Open in Browser

Open your browser and navigate to:

```
http://localhost:3000
```

## Configuration

### API URL Configuration

The UI connects to the API server. By default, it expects the API at `http://localhost:8080`.

**Option 1: Environment Variable**
```bash
API_URL=http://localhost:8080 ./target/release/csv-explorer-ui serve
```

**Option 2: Configuration File**
Edit `config.toml`:
```toml
[ui]
host = "0.0.0.0"
port = 3000
api_url = "http://localhost:8080"
```

### Custom Ports

**Change UI Port:**
```bash
# Via environment variable
UI_PORT=8000 ./target/release/csv-explorer-ui serve

# Or edit config.toml
[ui]
port = 8000
```

**Change API Port:**
```bash
# Edit config.toml
[api]
port = 9000
```

Then start with matching API URL:
```bash
API_URL=http://localhost:9000 ./target/release/csv-explorer-ui serve
```

## UI Features

### 1. Home Dashboard (`/`)

The landing page shows:
- **Statistics Cards**: Total sanads, transfers, seals, contracts
- **Chain Status**: Real-time status of all 5 chains (Bitcoin, Ethereum, Sui, Aptos, Solana)
- **Recent Activity**: Latest transfers and seal operations
- **Quick Search**: Search by ID or address

### 2. Sanads Management (`/sanads`)

**List View:**
- Browse all sanads with pagination
- Filter by chain (Bitcoin, Ethereum, Sui, Aptos, Solana)
- Filter by status (Active, Spent, Pending)
- Click any sanad to view details

**Detail View (`/sanads/:id`):**
- Full sanad information (ID, chain, owner, commitment, seal reference)
- Transfer history with links to individual transfers
- Associated seals with status
- Metadata display

### 3. Cross-Chain Transfers (`/transfers`)

**List View:**
- All transfers with source and destination chains
- Filter by status (Pending, In Progress, Completed, Failed)
- Transfer routes visualization

**Detail View (`/transfers/:id`):**
- Complete transfer information
- Visual progress timeline:
  1. Lock Submitted (source chain)
  2. Proof Generated
  3. Mint Completed (destination chain)
- Duration and timing information

### 4. Seals Inventory (`/seals`)

**List View:**
- All seals across all chains
- Filter by seal type (UTXO, Object, Resource, Nullifier, Account)
- Filter by status (Available, Consumed)

**Detail View (`/seals/:id`):**
- Seal information and type
- Chain-specific details
- Linked sanad (if consumed)
- Seal type explanation

### 5. Contracts Registry (`/contracts`)

- All deployed CSV contracts
- Filter by chain and type
- Contract versions and deployment status
- Addresses and deployment transactions

### 6. Chain Status (`/chains`)

- Real-time indexer status for all chains
- Latest block numbers
- Sync lag indicators
- Chain-specific information and seal types

### 7. Statistics (`/stats`)

- Aggregate statistics
- Transfer success rate
- Average transfer time
- Sanads distribution by chain
- Transfer volume by chain pair

### 8. Wallet (`/wallet`)

- Connect CSV wallet
- View wallet sanads
- Initiate transfers
- Balance display

## Development Mode

### Build UI from Source

```bash
# Build UI binary
cargo build --release -p csv-explorer-ui

# Or build everything
cargo build --workspace --release
```

### Hot Reload (Development)

For development with hot reload, use Dioxus CLI:

```bash
# Install dioxus-cli
cargo install dioxus-cli

# Run with hot reload
dx serve
```

### Check Build

```bash
# Check for compilation errors
cargo check -p csv-explorer-ui

# Run tests
cargo test -p csv-explorer-ui
```

## Troubleshooting

### UI Won't Start

```bash
# Check if port 3000 is in use
lsof -i :3000

# Kill existing process
kill $(lsof -t -i:3000)

# Restart UI
./target/release/csv-explorer-ui serve
```

### API Connection Error

If the UI shows "API server unreachable":

```bash
# 1. Verify API is running
curl http://localhost:8080/health

# 2. Check API URL configuration
echo $API_URL  # Should be http://localhost:8080

# 3. Restart UI with correct API URL
API_URL=http://localhost:8080 ./target/release/csv-explorer-ui serve
```

### UI Shows No Data

```bash
# 1. Check database has data
sqlite3 data/explorer.db "SELECT COUNT(*) FROM sanads;"

# 2. Verify API returns data
curl http://localhost:8080/api/v1/sanads

# 3. Check UI logs for errors
./target/release/csv-explorer-ui serve 2>&1 | tail -n 50
```

### CORS Issues

If you see CORS errors in browser console, the API should already have CORS enabled. Check:

```bash
# Test CORS
curl -H "Origin: http://localhost:3000" -H "Access-Control-Request-Method: GET" -X OPTIONS http://localhost:8080/api/v1/sanads -v
```

## Advanced Configuration

### Production Deployment

```bash
# 1. Build release binaries
cargo build --workspace --release

# 2. Configure production settings
cat > config.toml << EOF
[database]
url = "sqlite:///var/lib/csv-explorer/explorer.db"

[api]
host = "0.0.0.0"
port = 8080

[ui]
host = "0.0.0.0"
port = 3000
api_url = "http://api.production.com:8080"
EOF

# 3. Start services
./target/release/csv-explorer-api start &
./target/release/csv-explorer-ui serve &
```

### Docker Deployment

```bash
# Start all services with Docker Compose
docker compose up -d

# Access UI
open http://localhost:3000

# Check logs
docker compose logs -f ui
docker compose logs -f api
```

### Reverse Proxy (Nginx)

```nginx
server {
    listen 80;
    server_name explorer.example.com;

    # UI
    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    # API
    location /api/ {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
    }

    # GraphQL
    location /graphql {
        proxy_pass http://localhost:8080;
    }
}
```

## UI Architecture

The Dioxus Web UI consists of:

```
ui/
├── src/
│   ├── main.rs              # Entry point (serve/desktop commands)
│   ├── app.rs               # Root component with routing
│   ├── routes.rs            # Route definitions
│   ├── components/          # Reusable UI components
│   │   ├── mod.rs
│   │   ├── search.rs        # Global search
│   │   ├── chain_badge.rs   # Chain indicator
│   │   ├── status_badge.rs  # Status indicator
│   │   └── timeline.rs      # Transfer timeline
│   ├── pages/               # Page components
│   │   ├── home.rs          # Dashboard
│   │   ├── sanads.rs        # Sanads list
│   │   ├── sanad_detail.rs  # Sanad details
│   │   ├── transfers.rs     # Transfers list
│   │   ├── transfer_detail.rs # Transfer details
│   │   ├── seals.rs         # Seals list
│   │   ├── seal_detail.rs   # Seal details
│   │   ├── contracts.rs     # Contracts list
│   │   ├── stats.rs         # Statistics
│   │   ├── chains.rs        # Chain status
│   │   └── wallet.rs        # Wallet page
│   └── hooks/               # Custom hooks
│       ├── use_api.rs       # API client
│       └── use_wallet.rs    # Wallet connection
```

## API Integration

The UI uses `ApiClient` (from `hooks/use_api.rs`) to communicate with the backend:

```rust
// Example: Fetch sanads in a component
let api = use_resource(|| async move { ApiClient::new() });

let sanads = use_resource({
    let api = api.clone();
    move || async move {
        if let Some(client) = api.read().as_ref() {
            client.get_sanads(None, None, Some(10), None).await.ok()
        } else {
            None
        }
    }
});
```

## Performance Tips

1. **Pagination**: Use limit/offset for large datasets
2. **Filtering**: Apply filters to reduce data transfer
3. **Caching**: The UI caches responses where appropriate
4. **Lazy Loading**: Detail pages load data on-demand

## Next Steps

After getting the UI running:

1. **Explore the Data**: Browse sanads, transfers, and seals
2. **Test Filtering**: Use chain and status filters
3. **View Details**: Click into individual records
4. **Check Statistics**: View aggregate data on `/stats`
5. **Monitor Chains**: Check sync status on `/chains`

## Support

For issues or questions:

1. Check logs: `./target/release/csv-explorer-ui serve 2>&1`
2. Verify API: `curl http://localhost:8080/health`
3. Check database: `sqlite3 data/explorer.db`
4. Review docs: `TESTING.md`, `TEST_SETUP.md`
