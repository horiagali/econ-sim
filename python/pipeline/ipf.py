"""Iterative proportional fitting (raking) of sample weights to marginals.

Given a seed sample of records with categorical attributes and target totals
for one or more attributes (each a dict category → total), adjusts weights
multiplicatively until every marginal matches. Standard textbook IPF; used to
reweight the synthetic population to census county totals (ADR-0003).
"""
from __future__ import annotations


def ipf(records: list[dict], weights: list[float], margins: dict[str, dict], max_iter: int = 200, tol: float = 1e-9) -> tuple[list[float], int]:
    """Return (fitted weights, iterations). `margins` maps attribute → {category: target total}."""
    w = list(weights)
    for it in range(1, max_iter + 1):
        for attr, targets in margins.items():
            sums: dict = {}
            for r, wi in zip(records, w):
                sums[r[attr]] = sums.get(r[attr], 0.0) + wi
            for i, r in enumerate(records):
                cur = sums.get(r[attr], 0.0)
                tgt = targets.get(r[attr])
                if tgt is None or cur == 0.0:
                    continue
                w[i] *= tgt / cur
        # Convergence is checked on ALL margins after a full sweep.
        worst = 0.0
        for attr, targets in margins.items():
            for cat, tgt in targets.items():
                got = sum(wi for r, wi in zip(records, w) if r[attr] == cat)
                worst = max(worst, abs(got - tgt) / max(tgt, 1.0))
        if worst < tol:
            return w, it
    return w, max_iter


def integerise(weights: list[float]) -> list[int]:
    """Round weights to integers keeping the total (largest remainder, index tiebreak)."""
    total = round(sum(weights))
    floors = [int(x) for x in weights]
    rem = total - sum(floors)
    order = sorted(range(len(weights)), key=lambda i: (-(weights[i] - floors[i]), i))
    for i in order[:rem]:
        floors[i] += 1
    return floors
