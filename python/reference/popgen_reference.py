#!/usr/bin/env python3
"""Independent reference for the population generator (spec `society/population-generator`).

Written from the spec, standard library only, integer arithmetic only, so the
Rust generator can be compared with it record for record (AC-POP-07).
Randomness is the project's keyed ChaCha8 (`KeyedRng::draw`, ADR-0006),
re-implemented here and checked against the known-answer vectors of `econ-rng`.

    python python/reference/popgen_reference.py MARGINS.json --sample-scale 1000 --report
"""
from __future__ import annotations

import argparse
import json
import sys
import time

M32 = 0xFFFFFFFF
M64 = 0xFFFFFFFFFFFFFFFF
UNIT = 1_000_000  # weights are raked in millionths of a household
STREAM_POPULATION_GEN = 4
SEX_F, SEX_M = 0, 1
ROLE_HEAD, ROLE_PARTNER, ROLE_CHILD, ROLE_OTHER = 0, 1, 2, 3
TARGET_SWEEPS = 50  # two-way balancing of the person targets (Stage B, step 7a)
ADULT_AGE = 20  # years; heads and "an adult in the household" (spec, Stage A step 4)


# --- keyed ChaCha8 (ADR-0006) ------------------------------------------------

def _splitmix64(state: int) -> tuple[int, int]:
    state = (state + 0x9E3779B97F4A7C15) & M64
    z = state
    z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & M64
    z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & M64
    return state, z ^ (z >> 31)


def _derive_key(seed: int, stream: int) -> list[int]:
    st = seed ^ ((stream << 32) | stream)
    words = []
    for _ in range(4):
        st, v = _splitmix64(st)
        words += [v & M32, v >> 32]
    return words


def _chacha8_block(key: list[int], counter: int, stream: int) -> list[int]:
    init = [0x61707865, 0x3320646E, 0x79622D32, 0x6B206574, *key,
            counter & M32, (counter >> 32) & M32, stream & M32, (stream >> 32) & M32]
    x = list(init)

    def qr(a: int, b: int, c: int, d: int) -> None:
        x[a] = (x[a] + x[b]) & M32; x[d] ^= x[a]; x[d] = ((x[d] << 16) | (x[d] >> 16)) & M32  # noqa: E702
        x[c] = (x[c] + x[d]) & M32; x[b] ^= x[c]; x[b] = ((x[b] << 12) | (x[b] >> 20)) & M32  # noqa: E702
        x[a] = (x[a] + x[b]) & M32; x[d] ^= x[a]; x[d] = ((x[d] << 8) | (x[d] >> 24)) & M32  # noqa: E702
        x[c] = (x[c] + x[d]) & M32; x[b] ^= x[c]; x[b] = ((x[b] << 7) | (x[b] >> 25)) & M32  # noqa: E702

    for _ in range(4):
        qr(0, 4, 8, 12); qr(1, 5, 9, 13); qr(2, 6, 10, 14); qr(3, 7, 11, 15)  # noqa: E702
        qr(0, 5, 10, 15); qr(1, 6, 11, 12); qr(2, 7, 8, 13); qr(3, 4, 9, 14)  # noqa: E702
    return [(a + b) & M32 for a, b in zip(x, init)]


class Draw:
    """The generator for one (stream, tick, entity) context."""

    WORDS_PER_TICK = 1 << 20

    def __init__(self, seed: int, stream: int, tick: int, entity: int):
        self.key = _derive_key(seed, stream)
        self.entity = entity
        pos = tick * self.WORDS_PER_TICK
        self.block, self.index = pos // 16, pos % 16
        self.words = _chacha8_block(self.key, self.block, entity)

    def _u32(self) -> int:
        if self.index == 16:
            self.block += 1
            self.words = _chacha8_block(self.key, self.block, self.entity)
            self.index = 0
        v = self.words[self.index]
        self.index += 1
        return v

    def u64(self) -> int:
        lo = self._u32()
        return lo | (self._u32() << 32)

    def below(self, n: int) -> int:
        zone = M64 - (M64 % n)
        while True:
            v = self.u64()
            if v < zone:
                return v % n


def check_rng() -> None:
    """Known-answer vectors from crates/econ-rng (seed 42)."""
    got = [Draw(42, 0, 0, 0).u64(), Draw(42, 0, 7, 123_456).u64(), Draw(42, 2, 7, 123_456).u64()]
    want = [0x31159EF987C91AFC, 0x7A16F9E8D24CE604, 0x013C26072112EFA4]
    if got != want:
        raise AssertionError(f"ChaCha8 port does not match econ-rng: {[hex(g) for g in got]}")


# --- integer helpers ----------------------------------------------------------

def rdiv(p: int, d: int) -> int:
    """p / d rounded half away from zero (p >= 0, d > 0)."""
    q, r = divmod(p, d)
    return q + 1 if 2 * r >= d else q


def apportion(total: int, targets: list[int]) -> list[int]:
    """Split `total` in proportion to `targets` by largest remainder, ties to the lower index."""
    tsum = sum(targets)
    if tsum == 0:
        out = [0] * len(targets)
        if out:
            out[0] = total
        return out
    parts = [total * t // tsum for t in targets]
    order = sorted(range(len(targets)), key=lambda i: (-(total * targets[i] % tsum), i))
    for i in order[: total - sum(parts)]:
        parts[i] += 1
    return parts


def straddle(n: int, num: int, den: int, lo: int, hi: int) -> list[int]:
    """`n` integer sizes in [lo, hi] around the average num/den: both neighbours
    of a fractional average are present when n >= 2, the smaller sizes first."""
    if num <= lo * den:
        return [lo] * n
    if num >= hi * den:
        return [hi] * n
    f, frac = divmod(num, den)
    if frac == 0:
        return [f] * n
    if n == 1:
        return [f + 1 if 2 * frac >= den else f]
    n_hi = min(n - 1, max(1, rdiv(n * frac, den)))
    return [f] * (n - n_hi) + [f + 1] * n_hi


class GenError(ValueError):
    pass


# --- the generator -----------------------------------------------------------

def generate(m: dict, sample_scale: int, rng_seed: int = 42, max_household_size: int = 15,
             raking_tolerance_ppm: int = 1000, max_sweeps: int = 50, min_cell_records: int = 30) -> dict:
    if sample_scale <= 0:
        raise GenError("ZeroScale")
    if not m["counties"] or not m["age_bands"] or not m["size_classes"]:
        raise GenError("EmptyTable")
    for table in ("persons_private", "persons_collective"):
        if any(v < 0 for county in m[table] for sex in county for v in sex):
            raise GenError("NegativeCount")
    if any(v < 0 for county in m["households"] for v in county):
        raise GenError("NegativeCount")
    bands = [(b[0], b[1] if b[1] is not None else b[0] + 5) for b in m["age_bands"]]
    classes = [(c[0], c[1] if c[1] is not None else max_household_size) for c in m["size_classes"]]
    n_bands = len(bands)

    hh_weight: list[int] = []
    hh_county: list[int] = []
    hh_collective: list[int] = []
    p_household: list[int] = []
    p_age: list[int] = []
    p_sex: list[int] = []
    p_role: list[int] = []
    report = {"sweeps": 0, "converged": True, "max_error_ppm": 0, "unfitted_cells": 0}
    person_seq = 0
    seeds = []

    for ci, county in enumerate(m["counties"]):
        H = m["households"][ci]
        P = [v for sex in m["persons_private"][ci] for v in sex]      # [sex][band] flattened
        Q = [v for sex in m["persons_collective"][ci] for v in sex]
        p_total, h_total, q_total = sum(P), sum(H), sum(Q)
        lo = sum(n * c[0] for n, c in zip(H, classes))
        hi = sum(n * c[1] for n, c in zip(H, classes))
        if h_total == 0 or not lo <= p_total <= hi:
            raise GenError(f"InconsistentMargins: {county}")

        # Stage A, steps 1-2: households and their sizes.
        n_hh = max(1, rdiv(h_total, sample_scale))
        per_class = apportion(n_hh, H)
        # A size class with real households but no synthetic one hands its
        # target to the nearest class that has one (the smaller on a tie).
        Ht = list(H)
        for k in range(len(classes)):
            if Ht[k] > 0 and per_class[k] == 0:
                j = min((abs(i - k), i) for i in range(len(classes)) if per_class[i] > 0)[1]
                Ht[j] += Ht[k]
                Ht[k] = 0
        # Classes of one size have that size. The open classes must hold the
        # rest of the county's persons, so their sizes straddle the average
        # they need: then weights exist that give both the right number of
        # households and the right number of persons.
        sizes_of = [[classes[k][0]] * per_class[k] for k in range(len(classes))]
        open_ks = [k for k in range(len(classes)) if classes[k][1] > classes[k][0] and per_class[k] > 0]
        need = p_total - sum(classes[k][0] * Ht[k] for k in range(len(classes)) if k not in open_ks)
        for i, k in enumerate(open_ks):
            lo_k, hi_k = classes[k]
            later_min = sum(classes[j][0] * Ht[j] for j in open_ks[i + 1:])
            num = need - later_min
            if num <= hi_k * Ht[k] or i == len(open_ks) - 1:
                sizes_of[k] = straddle(per_class[k], num, Ht[k], lo_k, hi_k)
                break
            sizes_of[k] = [hi_k] * per_class[k]
            need -= hi_k * Ht[k]
        size: list[int] = []
        klass: list[int] = []
        for k in range(len(classes)):
            size += sizes_of[k]
            klass += [k] * per_class[k]
        members = sum(size)

        # Step 3: who the persons are. Each person: [sex, band, seq].
        persons = []
        for cell, n in enumerate(apportion(members, P)):
            for _ in range(n):
                persons.append((cell // n_bands, cell % n_bands, person_seq))
                person_seq += 1
        n_coll = max(1 if q_total > 0 else 0, rdiv(q_total, sample_scale))
        collective = []
        for cell, n in enumerate(apportion(n_coll, Q)):
            for _ in range(n):
                collective.append((cell // n_bands, cell % n_bands, person_seq))
                person_seq += 1

        # Step 5 (done first, the draws are per person): priority and exact age.
        prio: dict[int, int] = {}
        age: dict[int, int] = {}
        for sex, band, seq in persons + collective:
            d = Draw(rng_seed, STREAM_POPULATION_GEN, 0, seq)
            prio[seq] = d.u64()
            age[seq] = bands[band][0] * 12 + d.below((bands[band][1] - bands[band][0]) * 12)

        # Step 4: assembly, in keyed-random order.
        order = sorted(persons, key=lambda p: (prio[p[2]], p[2]))
        placed: set[int] = set()
        hh_members: list[list] = [[] for _ in range(n_hh)]   # (sex, age months, seq, role)
        heads = [p for p in order if bands[p[1]][0] >= ADULT_AGE][:n_hh]
        if len(heads) < n_hh:
            heads += [p for p in order if 15 <= bands[p[1]][0] < ADULT_AGE][: n_hh - len(heads)]
        if len(heads) < n_hh:
            taken = {p[2] for p in heads}
            heads += [p for p in order if p[2] not in taken][: n_hh - len(heads)]
        for h, p in enumerate(heads):
            hh_members[h].append([p[0], age[p[2]], p[2], ROLE_HEAD])
            placed.add(p[2])
        children = [p for p in order if p[2] not in placed and bands[p[1]][1] <= 15]
        eligible = [h for h in range(n_hh) if size[h] >= 2 and 20 * 12 <= hh_members[h][0][1] < 60 * 12]
        while children and eligible:
            still = []
            for h in eligible:
                if not children:
                    break
                if len(hh_members[h]) < size[h]:
                    p = children.pop(0)
                    hh_members[h].append([p[0], age[p[2]], p[2], ROLE_CHILD])
                    placed.add(p[2])
                if len(hh_members[h]) < size[h]:
                    still.append(h)
            eligible = still if children else []
        rest = [p for p in order if p[2] not in placed]
        ri = 0
        for h in range(n_hh):
            head_sex, head_age = hh_members[h][0][0], hh_members[h][0][1]
            while len(hh_members[h]) < size[h] and ri < len(rest):
                p = rest[ri]
                ri += 1
                a = age[p[2]]
                has_partner = any(x[3] == ROLE_PARTNER for x in hh_members[h])
                if a < ADULT_AGE * 12 and head_age - a >= 18 * 12:
                    role = ROLE_CHILD
                elif a >= ADULT_AGE * 12 and p[0] != head_sex and abs(a - head_age) <= 15 * 12 and not has_partner:
                    role = ROLE_PARTNER
                else:
                    role = ROLE_OTHER
                hh_members[h].append([p[0], a, p[2], role])
        size = [len(x) for x in hh_members]   # places that could not be filled do not exist
        by_cell: dict[int, dict[int, int]] = {}
        for h, mem in enumerate(hh_members):
            for sex, a, _seq, _role in mem:
                band = next(i for i, b in enumerate(bands) if b[0] * 12 <= a < b[1] * 12)
                cell = by_cell.setdefault(sex * n_bands + band, {})
                cell[h] = cell.get(h, 0) + 1
        seeds.append((H, P, Q, n_hh, klass, size, hh_members, by_cell, collective, age, Ht))

    # Stage B, step 7a: person targets. A county cell with people but no synthetic
    # record cannot be fitted, so each (sex, age band) total of the country is
    # shared among the county cells that do have records, in proportion to their
    # own census counts. A band with no record anywhere is added to the nearest
    # band of the same sex that has one (the younger on a tie).
    n_cells = 2 * n_bands
    national = [sum(seed[1][cell] for seed in seeds) for cell in range(n_cells)]
    covered = [[seed[1][cell] if cell in seed[7] else 0 for seed in seeds] for cell in range(n_cells)]
    for cell in range(n_cells):
        if national[cell] > 0 and sum(covered[cell]) == 0:
            sex, band = divmod(cell, n_bands)
            near = sorted((abs(b - band), b) for b in range(n_bands) if sum(covered[sex * n_bands + b]) > 0)
            if not near:
                raise GenError("EmptyTable")
            national[sex * n_bands + near[0][1]] += national[cell]
            national[cell] = 0
    # Balance the target table both ways (integer biproportional fitting): rows
    # keep the national (sex, age band) totals, columns keep each county's own
    # person total, which is what its households must hold. Counties win: the
    # last step makes every column exact.
    county_total = [sum(seed[1]) for seed in seeds]
    t = [[v * UNIT for v in row] for row in covered]
    for _ in range(TARGET_SWEEPS):
        for cell in range(n_cells):
            rs = sum(t[cell])
            if rs:
                t[cell] = [rdiv(v * national[cell] * UNIT, rs) for v in t[cell]]
        for ci in range(len(seeds)):
            cs = sum(t[cell][ci] for cell in range(n_cells))
            if cs:
                for cell in range(n_cells):
                    t[cell][ci] = rdiv(t[cell][ci] * county_total[ci] * UNIT, cs)
    targets = [[0] * len(seeds) for _ in range(n_cells)]   # [cell][county]
    for ci in range(len(seeds)):
        for cell, v in enumerate(apportion(county_total[ci], [t[cell][ci] for cell in range(n_cells)])):
            targets[cell][ci] = v
    for cell in range(n_cells):
        report["unfitted_cells"] += sum(1 for seed in seeds if seed[1][cell] > 0 and cell not in seed[7])

    for ci, county in enumerate(m["counties"]):
        H, P, Q, n_hh, klass, size, hh_members, by_cell, collective, age, Ht = seeds[ci]
        p_total, h_total, q_total = sum(P), sum(H), sum(Q)

        # Step 7b: integer raking of private households.
        w = [sample_scale * UNIT] * n_hh
        cons = []   # (target in real units, [(household, count)])
        for k in range(len(classes)):
            cons.append((Ht[k], [(h, 1) for h in range(n_hh) if klass[h] == k]))
        target = [targets[cell][ci] for cell in range(n_cells)]
        for cell in range(2 * n_bands):
            cons.append((target[cell], sorted(by_cell.get(cell, {}).items())))
        checked = [i for i, (t, mem) in enumerate(cons) if t >= min_cell_records * sample_scale]
        sweeps, worst = 0, 0
        for sweeps in range(1, max_sweeps + 1):
            for t, mem in cons:
                cur = sum(w[h] * n for h, n in mem)
                if t == 0 or cur == 0:
                    continue
                tu = t * UNIT
                for h, _n in mem:
                    w[h] = rdiv(w[h] * tu, cur)
            worst = 0
            for i in checked:
                t, mem = cons[i]
                cur = sum(w[h] * n for h, n in mem)
                worst = max(worst, abs(cur - t * UNIT) * 1_000_000 // (t * UNIT))
            if worst <= raking_tolerance_ppm:
                break
        else:
            report["converged"] = False
        report["sweeps"] = max(report["sweeps"], sweeps)
        report["max_error_ppm"] = max(report["max_error_ppm"], worst)

        # Step 8: integer weights with the exact household total.
        wi = [x // UNIT for x in w]
        order_h = sorted(range(n_hh), key=lambda h: (-(w[h] % UNIT), h))
        missing = h_total - sum(wi)
        if missing >= 0:
            for j in range(missing):
                wi[order_h[j % n_hh]] += 1
        else:
            for j in range(-missing):
                wi[order_h[n_hh - 1 - (j % n_hh)]] -= 1
        for h in range(n_hh):
            while wi[h] < 1:
                donor = max(range(n_hh), key=lambda x: (wi[x], -x))
                if wi[donor] <= 1:
                    raise GenError(f"InconsistentMargins: {county}")
                wi[donor] -= 1
                wi[h] += 1

        # Step 9: exact person total, by moving single units of weight between sizes.
        gap = p_total - sum(wi[h] * size[h] for h in range(n_hh))
        resid = [w[h] - wi[h] * UNIT for h in range(n_hh)]   # > 0: rounded down
        by_size: dict[int, list[int]] = {}
        for h in range(n_hh):
            by_size.setdefault(size[h], []).append(h)
        sizes_present = sorted(by_size)
        guard = 0
        while gap != 0:
            guard += 1
            if guard > 100_000:
                raise GenError(f"InconsistentMargins: {county}")
            up = gap > 0
            donors_sizes = sizes_present if up else sizes_present[::-1]
            done = False
            for sa in donors_sizes:
                donors = [h for h in by_size[sa] if wi[h] >= 2]
                if not donors:
                    continue
                if up:
                    cand = [sb for sb in sizes_present if sa < sb <= sa + gap]
                    sb = max(cand) if cand else None
                else:
                    cand = [sb for sb in sizes_present if sa + gap <= sb < sa]
                    sb = min(cand) if cand else None
                if sb is None:
                    continue
                donor = min(donors, key=lambda h: (resid[h], h))
                recv = max(by_size[sb], key=lambda h: (resid[h], -h))
                wi[donor] -= 1
                wi[recv] += 1
                resid[donor] += UNIT
                resid[recv] -= UNIT
                gap -= sb - sa
                done = True
                break
            if not done:
                raise GenError(f"InconsistentMargins: {county}")

        # Emit private households and persons.
        base = len(hh_weight)
        for h in range(n_hh):
            hh_weight.append(wi[h])
            hh_county.append(ci)
            hh_collective.append(0)
            for sex, a, _seq, role in hh_members[h]:
                p_household.append(base + h)
                p_age.append(a)
                p_sex.append(sex)
                p_role.append(role)

        # Collective records: one person each; weights exact per cell where a record exists.
        if collective:
            cw = [0] * len(collective)
            cells: dict[int, list[int]] = {}
            for j, (sex, band, _seq) in enumerate(collective):
                cells.setdefault(sex * n_bands + band, []).append(j)
            for cell, idx in sorted(cells.items()):
                for j, part in zip(idx, apportion(Q[cell], [1] * len(idx))):
                    cw[j] = part
            # persons of cells with no record go to the county's records, largest first
            leftover = q_total - sum(cw)
            order_c = sorted(range(len(cw)), key=lambda j: (-cw[j], j))
            for j, part in zip(order_c, apportion(leftover, [cw[j] or 1 for j in order_c])):
                cw[j] += part
            for j in range(len(cw)):
                while cw[j] < 1:
                    donor = max(range(len(cw)), key=lambda x: (cw[x], -x))
                    cw[donor] -= 1
                    cw[j] += 1
            for j, (sex, band, seq) in enumerate(collective):
                hh_weight.append(cw[j])
                hh_county.append(ci)
                hh_collective.append(1)
                p_household.append(len(hh_weight) - 1)
                p_age.append(age[seq])
                p_sex.append(sex)
                p_role.append(ROLE_HEAD)

    return {"hh_weight": hh_weight, "hh_county": hh_county, "hh_collective": hh_collective,
            "household_id": p_household, "age": p_age, "sex": p_sex, "role": p_role, "report": report}


def state_hash(pop: dict) -> int:
    """FNV-1a 64 over the tables in a fixed order (little-endian fields)."""
    h = 0xCBF29CE484222325

    def feed(data: bytes) -> None:
        nonlocal h
        for b in data:
            h = ((h ^ b) * 0x100000001B3) & M64

    feed(len(pop["hh_weight"]).to_bytes(4, "little"))
    for wgt, c, col in zip(pop["hh_weight"], pop["hh_county"], pop["hh_collective"]):
        feed(wgt.to_bytes(4, "little") + bytes([c, col]))
    feed(len(pop["household_id"]).to_bytes(4, "little"))
    for hid, a, s, r in zip(pop["household_id"], pop["age"], pop["sex"], pop["role"]):
        feed(hid.to_bytes(4, "little") + a.to_bytes(2, "little") + bytes([s, r]))
    return h


# --- measuring the fit (for the spec's tolerance table) -------------------------

def fit_errors(m: dict, pop: dict, sample_scale: int, min_cell_records: int = 30) -> dict:
    """Largest relative error per margin family, over cells with target >= min_cell_records * scale."""
    n_c, n_b, n_k = len(m["counties"]), len(m["age_bands"]), len(m["size_classes"])
    bands = [(b[0], b[1] if b[1] is not None else 10_000) for b in m["age_bands"]]
    classes = [(c[0], c[1] if c[1] is not None else 10_000) for c in m["size_classes"]]
    got_p = [[[0] * n_b for _ in range(2)] for _ in range(n_c)]
    got_q = [[[0] * n_b for _ in range(2)] for _ in range(n_c)]
    got_h = [[0] * n_k for _ in range(n_c)]
    size = [0] * len(pop["hh_weight"])
    for hid in pop["household_id"]:
        size[hid] += 1
    for hid, a, s in zip(pop["household_id"], pop["age"], pop["sex"]):
        band = next(i for i, b in enumerate(bands) if b[0] * 12 <= a < b[1] * 12)
        table = got_q if pop["hh_collective"][hid] else got_p
        table[pop["hh_county"][hid]][s][band] += pop["hh_weight"][hid]
    for h, wgt in enumerate(pop["hh_weight"]):
        if not pop["hh_collective"][h]:
            k = next(i for i, c in enumerate(classes) if c[0] <= size[h] <= c[1])
            got_h[pop["hh_county"][h]][k] += wgt
    out: dict[str, dict[str, float]] = {}

    def note(name: str, got: int, want: int) -> None:
        records = want // sample_scale   # expected synthetic records behind the cell
        if records < min_cell_records:
            return
        bucket = ">=1000 records" if records >= 1000 else ">=100 records" if records >= 100 else f">={min_cell_records} records"
        fam = out.setdefault(name, {})
        fam[bucket] = max(fam.get(bucket, 0.0), round(abs(got - want) / want, 5))

    want_p, want_h = m["persons_private"], m["households"]
    broad = [(0, 3), (3, 13), (13, n_b)]   # 0-14, 15-64, 65+
    for s in range(2):
        note("national sex", sum(got_p[c][s][b] for c in range(n_c) for b in range(n_b)),
             sum(want_p[c][s][b] for c in range(n_c) for b in range(n_b)))
    for b in range(n_b):
        note("national age band", sum(got_p[c][s][b] for c in range(n_c) for s in range(2)),
             sum(want_p[c][s][b] for c in range(n_c) for s in range(2)))
    for k in range(n_k):
        note("national size class", sum(got_h[c][k] for c in range(n_c)), sum(want_h[c][k] for c in range(n_c)))
    for c in range(n_c):
        for s in range(2):
            note("county x sex", sum(got_p[c][s]), sum(want_p[c][s]))
            for b in range(n_b):
                note("county x sex x age band", got_p[c][s][b], want_p[c][s][b])
        for lo, hi in broad:
            note("county x broad age", sum(got_p[c][s][b] for s in range(2) for b in range(lo, hi)),
                 sum(want_p[c][s][b] for s in range(2) for b in range(lo, hi)))
        for k in range(n_k):
            note("county x size class", got_h[c][k], want_h[c][k])
    exact = all(
        sum(got_h[c]) == sum(want_h[c])
        and sum(map(sum, got_p[c])) == sum(map(sum, want_p[c]))
        and sum(map(sum, got_q[c])) == sum(map(sum, m["persons_collective"][c]))
        for c in range(n_c)
    )
    return {"exact_county_totals": exact, "max_relative_error": out}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("margins")
    ap.add_argument("--sample-scale", type=int, required=True)
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--report", action="store_true", help="print fit errors instead of the tables")
    a = ap.parse_args()
    check_rng()
    with open(a.margins, encoding="utf-8") as f:
        m = json.load(f)
    t0 = time.time()
    pop = generate(m, a.sample_scale, a.seed)
    summary = {
        "sample_scale": a.sample_scale, "seed": a.seed,
        "households": len(pop["hh_weight"]), "persons": len(pop["household_id"]),
        "weighted_households": sum(w for w, c in zip(pop["hh_weight"], pop["hh_collective"]) if not c),
        "weighted_persons": sum(pop["hh_weight"][h] for h in pop["household_id"]),
        "state_hash": f"{state_hash(pop):016x}", "report": pop["report"],
        "seconds": round(time.time() - t0, 1),
    }
    if a.report:
        summary["fit"] = fit_errors(m, pop, a.sample_scale)
        print(json.dumps(summary, indent=2))
    else:
        print(json.dumps({"summary": summary, **{k: v for k, v in pop.items() if k != "report"}}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
