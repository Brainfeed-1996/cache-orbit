# Opérations

## Exécution locale

### Prérequis

- Rust ≥ 1.75
- Go ≥ 1.23
- Node.js ≥ 20
- Docker & Docker Compose
- Make

### Démarrage rapide

```bash
git clone git@github.com:org/cache-orbit.git
cd cache-orbit
make dev
```

### Architecture locale Docker

| Service | Port | Image |
|---------|------|-------|
| cache-engine | 6379, 9090 | rust:1.75-bookworm build |
| control-plane | 8080, 50051 | golang:1.23-alpine build |
| operator-ui | 3000 | node:20-alpine build |
| postgres | 5432 | postgres:16-alpine |
| nats | 4222 | nats:2-alpine |
| prometheus | 9090 | prom/prometheus |
| grafana | 3001 | grafana/grafana |

## Health checks

- `GET /healthz` (REST exhaust).
- `grpc.health.v1.Chec / Check` (gRPC).
- `GET /metrics` (Prometheus).

## Runbooks

### 1. Dégradation d'un nœud

1. Repérer le nœud dégradé sur la console (orange).
2. Invalider le nœud : `control-plane topo evict <node-id>`.
3. Vérifier la relocalisation des partitions dans la timeline d'invalidation.
4. Une fois le nœud réparé : `control-plane topo reinstate <node-id>`.

### 2. Baisse du hit ratio

1. Vérifier la fréquence d'invalidation (spike ?).
2. Identifier les clés éliminées sans raison (TTL court ?).
3. Ajuster `idle_ttl` via config du cluster.
4. Si persistant : vérifier le traçage `stale reads` dans Grafana (upper bound `stale.*`).

### 3. Hot-key surchargé

1. Repérage dans la heatmap (rouge).
2. Le système clone automatiquement la clé en réplica locale.
3. Si P99 > 15 ms, vérifier le `circuit_breaker` côté backend.

### 4. Remplacement de PostgreSQL

1. Spoonner le nouveau cluster : `pg_basebackup -h replica -D /var/lib/postgresql/data`.
2. Mettre à jour la config `config.yaml` du control plane.
3. Redémarrer le control plane :
   ```bash
   control-plane --config /etc/cache-orbit/config.yaml
   ```
4. Valider la migration : `control-plane topo check-consistency`.

## Monitoring

### Alertes clés

```yaml
groups:
  - name: cache_orbit.rules
    rules:
      - alert: HighLatencyP99
        expr: cache_node_p99_latency_ms > 8
        for: 2m
      - alert: LowHitRatio
        expr: cache_node_hit_ratio < 0.92
        for: 5m
      - alert: StaleCluster
        expr: cache_node_staleness_ms > 750
        for: 1m
      - alert: NodeUnreachable
        expr: up{job="cache-node"} == 0
        for: 30s
      - alert: HotKeyDetected
        expr: cache_node_hot_qps > 50000
        for: 1m
```

### Logs

Format structuré JSON, niveaux :

- `trace` : requêtes individuelles (désactivé en prod par défaut)
- `debug` : détection d'événements (hit, miss, invalidation)
- `info` : changements de topologie, hot-keys
- `warn` : erreurs récupérables, timeouts
- `error` : arrêt nœud, corruption

## Sauvegarde

- **Config** : sauvegardée automatiquement dans PostgreSQL (table `topology_snapshots`).
- **Données** : les données de cache sont *volatiles*. Un `snapshot` peut être déclenché via `control-plane snapshot create`.
