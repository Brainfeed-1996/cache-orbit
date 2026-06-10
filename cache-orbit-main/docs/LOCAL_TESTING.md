# Cache Orbit — Vérification Locale Windows

## Prérequis vérifiés sur ce poste

- Go 1.26.3 ✓
- Docker Desktop ✓
- PowerShell ✓

## Option 1 — Vérification rapide (contrôle-plan Go seul)

Cette option lance uniquement le control-plane en standalone, sans dépendances externes. La UI Next.js peut se connecter à http://localhost:8080.

```powershell
cd src\control-plane
go mod tidy
go run .
```

Puis dans un second terminal, lancez l'UI :
```powershell
cd src\operator-ui
npm install
npm run dev
```

## Option 2 — Stack complète Docker

```powershell
make dev
```

Vérifications :
- UI opérateur : http://localhost:3000
- Control plane (REST) : http://localhost:8080/health
- Cache engine (RESP) : localhost:6379
- Prometheus : http://localhost:9091
- Grafana : http://localhost:3001
- NATS Monitoring : http://localhost:8222

## Option 3 — Benchmark rapide

```powershell
cd src\control-plane
go run . --benchmark read-heavy --requests 50000 --concurrency 50
```

## Option 4 — Tests Rust après installation de cargo

```powershell
# Si Rust n'est pas installé, exécutez d'abord :
choco install rust
# Puis :
cd src\cache-engine
cargo test
```

## Arrêt

```powershell
make dev-stop    # Docker
# ou pour le control-plane standalone :
Ctrl+C dans le terminal
```
