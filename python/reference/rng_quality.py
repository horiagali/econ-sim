"""Statistical quality battery for the fast keyed hash (ADR-0006 amendment).

Re-implements `KeyedRng::fast_u64` in NumPy (checked against FAST_KAT) and runs
uniformity, bit balance, neighbour avalanche, neighbour bit correlation, serial
correlation, low-p Bernoulli co-occurrence and a 2-D serial test along each
input axis (entity, tick, k, seed). Pass = p-values not tiny, |z| < ~4.5.

Also compares two stronger variants ("gamma", "double") that were candidates
if the current mixer had failed. Needs numpy + scipy; ~5 min.
PractRand/TestU01 were not reachable from the sandbox; for an extra check pipe
raw output into PractRand on a desktop.
"""
import numpy as np, sys
from scipy import stats
np.seterr(over='ignore')
U=np.uint64
G=U(0x9E3779B97F4A7C15)
def mix(z):
    z=z+G
    z=(z^(z>>U(30)))*U(0xBF58476D1CE4E5B9)
    z=(z^(z>>U(27)))*U(0x94D049BB133111EB)
    return z^(z>>U(31))
def fast(seed,stream,tick,entity,k,variant="cur"):
    seed=np.asarray(seed,dtype=U); stream=np.asarray(stream,dtype=U); tick=np.asarray(tick,dtype=U); entity=np.asarray(entity,dtype=U); k=np.asarray(k,dtype=U)
    st=seed^U(0xD1B54A32D192ED03)
    st=mix(st^stream)
    st=mix(st^((tick<<U(32))|k))
    if variant=="cur": return mix(st^entity)
    if variant=="gamma": return mix(st^(entity*G))
    if variant=="double": return mix(mix(st^entity))
assert int(fast(42,0,7,123456,0))==744824335102330087, int(fast(42,0,7,123456,0))
print("KAT ok")

def battery(name, out_fn):
    res={}
    N=1<<22
    e=np.arange(N,dtype=U)
    x=out_fn(e)
    # 1. uniformity of top 16 bits
    c=np.bincount((x>>U(48)).astype(np.int64),minlength=65536)
    res["chi2_top16"]=stats.chisquare(c).pvalue
    # low 16 bits
    c=np.bincount((x&U(0xffff)).astype(np.int64),minlength=65536)
    res["chi2_low16"]=stats.chisquare(c).pvalue
    # 2. per-bit balance (min p over 64 bits, Bonferroni)
    bits=np.array([((x>>U(b))&U(1)).sum() for b in range(64)],dtype=float)
    z=(bits-N/2)/np.sqrt(N/4); res["bitbal_minp*64"]=min(1,2*stats.norm.sf(np.abs(z)).min()*64)
    # 3. avalanche between neighbours: hamming(x_e ^ x_{e+1}) ~ Bin(64,.5)
    h=np.unpackbits((x[1:]^x[:-1]).view(np.uint8)).reshape(-1,64).sum(1)
    hc=np.bincount(h,minlength=65); exp=stats.binom.pmf(np.arange(65),64,.5)*len(h)
    m=exp>5; res["avalanche_nbr"]=stats.chisquare(hc[m],exp[m]*hc[m].sum()/exp[m].sum()).pvalue
    # 4. pairwise bit correlation between x_e and x_{e+1}: 64x64 matrix of P(bit_i(e) xor bit_j(e+1))
    B=np.unpackbits(x.view(np.uint8)).reshape(-1,64).astype(np.int8)
    n2=1<<20
    A=B[:n2].astype(np.float32)*2-1; C=B[1:n2+1].astype(np.float32)*2-1
    corr=(A.T@C)/n2; zmax=np.abs(corr).max()*np.sqrt(n2)
    res["nbr_bitcorr_maxz"]=zmax  # expect ~ 4.5 for 4096 tests
    # 5. uniform serial correlation lag 1, 2, 1000
    u=(x>>U(11)).astype(np.float64)/2**53
    for lag in (1,2,1024):
        r=np.corrcoef(u[:-lag],u[lag:])[0,1]; res[f"serialcorr_lag{lag}_z"]=r*np.sqrt(N)
    # 6. low-p Bernoulli neighbour co-occurrence (labour separation-like)
    p=0.015; hit=u<p; j=(hit[1:]&hit[:-1]).sum(); expj=p*p*(N-1)
    res["bern_pair_z"]=(j-expj)/np.sqrt(expj)
    # 7. 2D serial test on top 8 bits of consecutive pairs
    a=(x>>U(56)).astype(np.int64); pairs=a[:-1:2]*256+a[1::2]
    res["serial2d_p"]=stats.chisquare(np.bincount(pairs,minlength=65536)).pvalue
    print(f"--- {name}")
    for k_,v in res.items(): print(f"  {k_:22s} {v:.4g}")
    return res

for variant in ["cur","gamma","double"]:
    battery(f"{variant}: sequence over entity (seed42,tick7,k0)", lambda e: fast(42,0,7,e,0,variant))
    battery(f"{variant}: sequence over tick (entity 5)", lambda e: fast(42,0,e&U(0xffffffff),5,0,variant))
    battery(f"{variant}: sequence over k (entity 5)", lambda e: fast(42,0,3,5,e&U(0xffffffff),variant))
    battery(f"{variant}: sequence over seed", lambda e: fast(e,0,3,5,0,variant))
