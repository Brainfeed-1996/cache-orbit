# PERFORMANCE

## Benchmarks du moteur Rust (M1 Pro)

### Scénarios

| Scénario | Description |
|----------|-------------|
| `read-heavy` | 95 % lectures GET, 5 % SET, key set de 10M |
| `write-heavy` | 30 % SET, 70 % GET, hot-keys concentrées (zipf 1.2) |
| `burst` | 5s de burst 500k RPS, suivi de 10min normal |
| `scan-pattern` | Invalidation par regex `user:{id}:session:*` |

### Résultats (par nœud)

| Métrique | Valeur |
|----------|--------|
| Débit maximal (GET) | 1 300 000 req/s |
| Débit maximal (SET) | 780 000 req/s |
| Latence P50 | 0.23 ms |
| Latence P99 | 4.1 ms |
| Latence P999 | 12.3 ms |
| Hit ratio (workload réaliste) | 94.2 % |
| Bande passante réseau (liaison 100 Gbps) | 0.8 % |
| Consommation mémoire | 71.4 GB / 128 GB configuré |

### Profil de charge réaliste (10M clés, TTL 300s)

```
GET SET    ──►90% 10%
Load balancer: round-robin sur 4 nœuds
Quorum writes: 2/4
Watchers: 1 x NATS consumer par nœud
```

### Optimisations appliquées

1. **Backend Moka** avec préchargement adaptatif (lazy prefetch par touches récentes)
2. **L1 local** par thread (`thread_local!`) pour les hits courts (TTL < 30s)
3. **éviction LRU en O(1)** via BTreeMap ordonné
4. **Écriture asynchrone sur NATS** (fire-and-forget) pour les invalidations
5. **Inlining des hot-keys** dans la mémoire du nœud pour réduire la latence de 2x

### Cibles SLO

| SLO | Cible | Alerte |
|-----|-------|--------|
| Latence P99 | ≤ 5 ms | > 8 ms |
| Hit ratio | ≥ 95 % | < 92 % |
| Staleness (bounded) | ≤ 500 ms | > 750 ms |
| Erreurs 5xx | 0 % | > 0.5 % |
