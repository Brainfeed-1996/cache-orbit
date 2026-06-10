# CONTRIBUTING

## Standards de code

### Rust (cache-engine)

- Formatter : `cargo fmt`
- Linter : `cargo clippy -- -D warnings`
- Pas d' `unsafe` sans justification ADR.
- Utiliser `thiserror` pour les erreurs métier.

### Go (control-plane)

- Linter : `golangci-lint`
- Formatter : `go fmt`
- Interface-first design : le handler gRPC reçoit une interface.

### TypeScript (operator-ui)

- Formatter : `prettier --check`
- Linter : `eslint src`
- Composants fonctionnels + hooks (`useState`, `useEffect`, `useSWR` pour données externes).

## Processus de code review

1. Ouvrir une PR avec un template (problème, solution, risque).
2. Au moins 1 reviewer pour chaque composant.
3. CI : lint + build + tests.
4. Merge après approbation.

## Architecture Decision Records (ADR)

Créer un fichier `docs/adr/NNN-slug.md` pour toute décision architecturale majeure. Format :

```markdown
# ADR 001: Choix de Rust pour le moteur de cache

## Statut
Accepté

## Contexte
Besoin d'un moteur de cache à latence <5ms P99.

## Décision
Implémenter le moteur en Rust.

## Conséquences
- Meilleure performance prédictible
- Courbe d'apprentissage plus forte
```

## Testing

- `cargo test` (unit + intégration dans Docker)
- `go test` (mocks interfaces gRPC)
- `npm test` (Jest + Testing Library)

## Commit

Convention : `type(scope): description`

```
feat(cache): add bounded staleness support
fix(topology): handle partition rolling during rebalance
chore(ci): bump rust toolchain to 1.75
```
