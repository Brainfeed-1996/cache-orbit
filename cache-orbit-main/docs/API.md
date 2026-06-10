# API

Cache Orbit expose deux types d'APIs : **REST (JSON)** pour les opérateurs et clients externes, **gRPC** pour le contrôle de topologie à haute performance.

## Base URL (local)

```
http://localhost:8080
http://localhost:50051 (gRPC)
```

## Éléments communs

```json
{
  "requestId": "uuid",
  "nodeId": "us-east-1-node-4",
  "timestampMs": 1718018394000
}
```

## RESP (Cache Engine)

Compatible Redis RESP2, sous-ensemble :

```
GET <key>
SET <key> <value> [EX <seconds>] [TAGS <tag1> <tag2> ...]
DEL <key>
PING
STATS
FLUSHALL
CONFIG GET <key>
```

Exemple :

```
> SET mykey "hello" EX 300 TAGS user:123
"OK"
> GET mykey
"hello"
```

### Réponse STATS

```json
{
  "hits": 1284300,
  "misses": 78100,
  "hit_ratio": 94.28,
  "p50_latency_ms": 0.23,
  "p99_latency_ms": 4.1,
  "capacity_bytes": 71400000000,
  "keys_count": 45230000,
  "timestamp_ms": 1718018394000
}
```

## REST API (Operator)

### Topologie

```
GET /api/v1/topology
```

Réponse 200 :

```json
{
  "version": "120",
  "nodes": [
    {
      "nodeId": "node-1",
      "datacenter": "eu-west-1",
      "state": "healthy",
      "lastHeartbeatMs": 1718018300000
    }
  ],
  "partitions": [
    {
      "id": 0,
      "primary": "node-1",
      "replicas": ["node-2"]
    }
  ]
}
```

### Invalidation

```
POST /api/v1/invalidation
```

```json
{
  "key": "user:123:profile",
  "scope": "local",
  "forceFlush": false
}
```

### Stats

```
GET /api/v1/stats?nodeId=node-1
```

```json
{
  "hitRatio": 94.2,
  "p50LatencyMs": 0.23,
  "p99LatencyMs": 4.1,
  "qps": 450000,
  "staleKeys": 120
}
```

### Benchmarks (bench)

```
POST /api/v1/bench
```

```json
{
  "scenario": "read-heavy",
  "rps": 500000,
  "durationSeconds": 10,
  "concurrency": 128
}
```

Réponse (async) :

```json
{
  "benchId": "bench-2024-06-10-uuid",
  "status": "running"
}
```

Résultat :

```
GET /api/v1/bench/bench-2024-06-10-uuid/result
```

## gRPC API

Importez le proto via :

```bash
protoc --go_out=. --go_opt=paths=source_relative \
       --go-grpc_out=. --go-grpc_opt=paths=source_relative \
       proto/control.proto
```

### Invalidation de clé

```go
resp, err := client.InvalidateKey(ctx, &controlv1.InvalidateKeyRequest{
    Key:         "user:123:profile",
    NodeId:      "node-1",
    ForceFlush:  true,
})
```

### Health check

```go
resp, err := client.HealthCheck(ctx, &controlv1.HealthCheckRequest{})
// resp.GetHealthy(): true
// resp.GetUptimeMs(): "120034000"
```

## Codes d'erreur

| Code | Signification |
|------|---------------|
| 200 | Succès |
| 400 | Requête invalide (paramètres manquants, TTL négatif, pattern non valide) |
| 404 | Clé ou nœud introuvable |
| 409 | Conflit de version topologie (retry avec la dernière version) |
| 429 | Rate limiting (hot-key protection) |
| 500 | Erreur interne (retry avec backoff exponentiel) |
| 503 | Nœud indisponible (circuit breaker activé) |

## Limites de débit

Par défaut en local :

- REST : 0 RPS limit (désactivé en dev)
- gRPC : 50k req/s par flux, throttle à 1M req/s cluster-wide
- Hot-key circuit breaker : 5009 req/s par clé avant limitation

## Authentification

Production :

- mTLS obligatoire entre control plane et nœuds,
- token bearer pour l'endpoint REST (`Authorization: Bearer <token>`),
- RBAC via policy attachée au token (lecture / écriture / invalidation / admin).
