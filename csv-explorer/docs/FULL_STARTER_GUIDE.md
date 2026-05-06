# CSV Explorer - Full Functional Guide ✅

## 🚀 Quick Start (One Command)

```bash
./start.sh
```

**Then open:** http://localhost:3000

That's it! Everything starts automatically.

---

## 📊 What's Running

### 1. API Server (Port 8080)
- REST API for all data
- GraphQL endpoint
- Health monitoring

### 2. Web UI (Port 3000)
- Dashboard with statistics
- Browse sanads, transfers, seals
- Real-time data from API

### 3. Database (SQLite)
- Pre-seeded with test data
- 8 sanads across 5 chains
- 7 cross-chain transfers
- 11 seals
- 11 contracts

---

## 🌐 Web UI Pages

### Home (`http://localhost:3000`)
Dashboard showing:
- **4 Stats Cards**: Sanads (8), Transfers (7), Seals (11), Contracts (11)
- **Chain Status**: Bitcoin, Ethereum, Sui, Aptos, Solana
- Click navigation to explore

### Sanads (`/sanads`)
Table with all sanads:
- Sanad ID (clickable)
- Chain (Bitcoin, Ethereum, etc.)
- Owner address
- Status (active/spent/pending)
- Transfer count

### Transfers (`/transfers`)
Cross-chain transfers:
- Transfer ID
- Route (e.g., bitcoin → ethereum)
- Status (completed/in_progress/pending/failed)
- Creation date

### Seals (`/seals`)
Seal inventory:
- Seal ID
- Chain
- Type (UTXO/Object/Resource/Nullifier/Account)
- Status (available/consumed)
- Block height

### Stats (`/stats`)
Detailed statistics:
- Transfer success rate: **80.0%**
- Average transfer time: **4.0 hours**
- Sanads by chain breakdown
- Transfers by chain pair
- Active seals by chain

---

## 🔧 Manual Start (if needed)

### Terminal 1: API Server
```bash
./target/release/csv-explorer-api start
```

### Terminal 2: Web UI
```bash
python3 -m http.server 3000 --directory ui/web
```

### Terminal 3: Verify
```bash
# Test API
curl http://localhost:8080/health

# Test UI
curl http://localhost:3000 | head -n 5
```

---

## 📡 API Endpoints

All endpoints are fully functional:

```bash
# Health
curl http://localhost:8080/health

# Statistics
curl http://localhost:8080/api/v1/stats

# Sanads
curl http://localhost:8080/api/v1/sanads?limit=10

# Transfers
curl http://localhost:8080/api/v1/transfers?limit=10

# Seals
curl http://localhost:8080/api/v1/seals?limit=10

# Contracts
curl http://localhost:8080/api/v1/contracts?limit=10

# Chains
curl http://localhost:8080/api/v1/chains

# GraphQL
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ stats { totalSanads totalTransfers } }"}'
```

---

## 💾 Database

### Location
```
data/explorer.db
```

### Query Directly
```bash
sqlite3 data/explorer.db

# List sanads
SELECT id, chain, status FROM sanads LIMIT 5;

# Count transfers by status
SELECT status, COUNT(*) FROM transfers GROUP BY status;

# View seals
SELECT id, chain, seal_type, status FROM seals;

# Exit
.quit
```

---

## 🎯 Test Data Included

### Sanads (8 total)
| Chain | Count | Status |
|-------|-------|--------|
| Bitcoin | 2 | Active |
| Ethereum | 2 | 1 Spent, 1 Active |
| Sui | 2 | 1 Active, 1 Pending |
| Aptos | 1 | Active |
| Solana | 1 | Active |

### Transfers (7 total)
- **Completed**: 4 (BTC→ETH, ETH→APT, APT→SOL, APT→ETH)
- **In Progress**: 1 (ETH→SUI)
- **Pending**: 1 (SOL→BTC)
- **Failed**: 1 (ETH→SUI)

### Seals (11 total)
- **Available**: 7
- **Consumed**: 4
- **Types**: UTXO, Tapret, Object, Resource, Nullifier, Account

### Contracts (11 total)
- **Active**: 8
- **Deprecated**: 1
- **Error**: 1

---

## 🛑 Stop Services

```bash
# Stop everything
pkill -f csv-explorer-api
pkill -f "http.server"

# Or use PIDs from start.sh output
kill <API_PID> <UI_PID>
```

---

## 📝 Logs

```bash
# API logs
tail -f /tmp/api.log

# UI logs
tail -f /tmp/ui.log
```

---

## ⚙️ Configuration

### Edit Config
```bash
nano config.toml
```

Key settings:
```toml
[database]
url = "sqlite://data/explorer.db"

[api]
port = 8080

[ui]
host = "0.0.0.0"
port = 3000
api_url = "http://localhost:8080"
```

### Custom Ports
```bash
# Change UI port
UI_PORT=8000 python3 -m http.server 8000 --directory ui/web

# Change API port (edit config.toml first)
[api]
port = 9000
```

---

## 🔍 Troubleshooting

### Port Already in Use
```bash
# Find what's using the port
lsof -i :3000
lsof -i :8080

# Kill it
kill $(lsof -t -i:3000)
kill $(lsof -t -i:8080)
```

### UI Not Loading
```bash
# Check if serving correctly
curl http://localhost:3000

# Should return HTML
# If not, restart:
pkill -f "http.server"
python3 -m http.server 3000 --directory ui/web &
```

### API Not Responding
```bash
# Check health
curl http://localhost:8080/health

# Restart
pkill -f csv-explorer-api
./target/release/csv-explorer-api start &
```

### Database Issues
```bash
# Recreate from seed
rm data/explorer.db
sqlite3 data/explorer.db < storage/src/schema.sql
sqlite3 data/explorer.db < storage/src/seed.sql
```

---

## ✅ Verification Checklist

Run these to verify everything works:

```bash
# 1. API Health
curl http://localhost:8080/health
# Expected: {"service":"csv-explorer-api","status":"ok"}

# 2. Statistics
curl http://localhost:8080/api/v1/stats | python3 -m json.tool
# Expected: JSON with total_sanads: 8

# 3. Web UI
curl http://localhost:3000 | grep "CSV Explorer"
# Expected: <title>CSV Explorer</title>

# 4. Database
sqlite3 data/explorer.db "SELECT COUNT(*) FROM sanads;"
# Expected: 8
```

---

## 📚 Documentation Files

- `README.md` - Project overview
- `WEB_UI_WORKING.md` - Web UI details
- `TESTS_PASS.md` - Test results
- `SETUP_COMPLETE.md` - Setup summary
- `TESTING.md` - Testing guide

---

## 🎉 You're Ready!

**Just run:**
```bash
./start.sh
```

**Then open:**
```
http://localhost:3000
```

**Everything is working:**
- ✅ API Server (port 8080)
- ✅ Web UI (port 3000)
- ✅ Database seeded
- ✅ All endpoints tested
- ✅ Test data loaded

**Enjoy exploring!** 🚀
