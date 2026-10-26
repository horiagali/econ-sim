"""Independent reference implementation of Godley & Lavoie model PC.

Source: Godley & Lavoie (2007), *Monetary Economics*, ch. 4 ("portfolio
choice"). Written from the textbook equations, not from the Rust code
(ADR-0010 differential testing). Equations, with r the bill rate set by the
central bank and interest paid on last period's stocks at last period's rate:

    Y   = C + G
    YD  = Y - T + r(-1) * Bh(-1)
    T   = theta * (Y + r(-1) * Bh(-1))
    V   = V(-1) + (YD - C)
    C   = alpha1 * YD + alpha2 * V(-1)
    Hh  = V - Bh
    Bh  = V * (lambda0 + lambda1 * r - lambda2 * YD / V)
    dBs = G + r(-1) * Bs(-1) - (T + r(-1) * Bcb(-1))
    Bcb = Bs - Bh
    Hs  = Hs(-1) + dBcb            (= Hh: the redundant equation)

Shared conventions with the Rust core (same as the SIM reference):
  * money is an integer number of bani;
  * Y is solved from the closed form and rounded half away from zero:
        Y = (G + alpha2 * V(-1) + alpha1 * (1 - theta) * iH) / (1 - alpha1 * (1 - theta))
    where iH = round(r(-1) * Bh(-1)) is the interest households receive;
  * every product of a rate and an amount is rounded half away from zero;
  * C = Y - G (goods market clears exactly), so the behavioural consumption
    function holds up to one bano of rounding;
  * Bh = round(lambda0 * V + lambda1 * r * V - lambda2 * YD), the demand for
    bills multiplied out (no division by V, which is zero at the start);
  * the central bank remits its interest income to the government, so
    dBs = G + iH - T.
The textbook experiment is used: r rises from 2.5% to 3.5% at tick RATE_TICK.
"""
from __future__ import annotations

from dataclasses import dataclass

from sim_reference import round_half_away

RATE_TICK = 100


@dataclass(frozen=True)
class Params:
    g: int = 20_000_000 * 100  # 20 million lei per tick, in bani
    theta: float = 0.2
    alpha1: float = 0.6
    alpha2: float = 0.4
    lambda0: float = 0.635
    lambda1: float = 5.0
    lambda2: float = 0.01
    r: float = 0.025
    r_after: float = 0.035


def run(ticks: int, p: Params = Params()) -> list[dict[str, int]]:
    rows = []
    v = bh = bs = bcb = hh = 0
    r_prev = p.r
    for tick in range(ticks):
        r = p.r_after if tick >= RATE_TICK else p.r
        ih = round_half_away(r_prev * float(bh))   # interest to households
        icb = round_half_away(r_prev * float(bcb))  # interest to the central bank
        y = round_half_away(
            (float(p.g) + p.alpha2 * float(v) + p.alpha1 * (1.0 - p.theta) * float(ih))
            / (1.0 - p.alpha1 * (1.0 - p.theta)))
        t = round_half_away(float(y + ih) * p.theta)
        yd = y + ih - t
        c = y - p.g
        v_new = v + yd - c
        bh_new = round_half_away(p.lambda0 * float(v_new) + p.lambda1 * r * float(v_new) - p.lambda2 * float(yd))
        hh_new = v_new - bh_new
        # Government budget: spending and interest, less taxes and the
        # central bank's profit (its interest income, remitted).
        bs_new = bs + (p.g + ih + icb) - (t + icb)
        bcb_new = bs_new - bh_new
        hs_new = bcb_new  # the central bank's only asset is bills, its only liability cash
        assert hs_new == hh_new, "redundant equation: cash supplied = cash held"
        assert v_new == bs_new, "household wealth = government debt"
        assert abs(c - (p.alpha1 * yd + p.alpha2 * v)) <= 1.0
        v, bh, bs, bcb, hh = v_new, bh_new, bs_new, bcb_new, hh_new
        r_prev = r
        rows.append({"tick": tick, "Y": y, "T": t, "YD": yd, "C": c, "G": p.g, "V": v,
                     "BH": bh, "HH": hh, "BS": bs, "BCB": bcb, "IH": ih, "ICB": icb})
    return rows


if __name__ == "__main__":
    for row in run(5):
        print(row)
