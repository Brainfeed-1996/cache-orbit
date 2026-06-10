# Roadmap

## Court terme (Semaines 1-4)

- [x] Modélisation de topologie PostgreSQL + API de lecture
- [x] Moteur Rust GET/SET/DEL avec stats Prometheus
- [ ] CLI Go + gRPC pour configurer les nœuds
- [ ] Tests unitaires + intégration CI
- [ ] Benchmarks de charge minimal (3 scénarios)

## Moyen terme (Mois 2-3)

- [ ] Invalidation par pattern (`SCAN`) avec garanties de cohérence configurable
- [ ] Détection des hot-keys et réplication auto
- [ ] Console opérateur temps réel (WebSocket)
- [ ] Multi-tier (L1 + L2) avec tiering automatique par caractéristique d'accès
- [ ] Mode strong consistency avec quorum

## Long terme (Mois 4-6)

- [ ] Support multi-région Géolocalisation + routing basé sur latence
- [ ] Cache-aside orchestré (refresh on miss avec backfill)
- [ ] Observabilité distribuée (OTel, traces Span)
- [ ] Réplication active-active multi-région
- [ ] API publique et SDK client (Go, Python, TS)

## Philosophie de roadmap

Chaque ajout de fonctionnalité est décomposé en feature-flag derrière une interface clairement typée, sans breaking change pour les nœuds existants. La compatibilité gRPC est maintenue version par version via `major` proto.
