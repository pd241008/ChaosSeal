# CEP End-to-End Security Composition & Dynamic Membership Cost Model

**Supplementary material addressing the two Tier-3 gaps flagged for strengthening:
(1) composing the BEE collusion-resistance argument with the data-layer
IND-CPA/INT-CTXT argument into a single end-to-end proof; (2) bounding the cost
of dynamic swarm membership (satellites joining post-deployment).**

---

## Part A — End-to-End Composition of the Two Security Arguments

### A.1 The two component arguments

**Component 1 — BEE revocation (broadcast layer).** The BEE layer is a
subset-difference-style key tree over the $N=1024$ swarm. When $r$ receivers are
revoked, the transmitter emits a covering set of subset keys such that:

1. **Collusion-resistance (forward secrecy on revocation):** any coalition of
   non-revoked receivers holding their assigned leaf keys cannot recover any key
   assigned to a revoked receiver, and cannot recover the revocation-update
   secret $\mathsf{msk}$. This holds provided the XOR-tree key derivation
   provides $t$-wise independence: a coalition of $c < r$ non-revoked leaves
   learns at most the keys on the root-to-leaf path above non-revoked nodes, and
   by the no-revoked-ancestor property of the covering set, no revoked leaf's
   secret is on any such path.
2. We model revocation-vs-time: revocation is *instantaneous and unilateral* —
   the transmitter re-keys and broadcasts the covering set; no interaction with
   the revoked node is required, which is the whole point vs TLS's per-pair
   handshake.

**Component 2 — data transport (chaos-derived HKDF→AES-256-GCM + HMAC-EtM).**
Each epoch yields a session key via the deterministic chaotic pendulum
(discussed below); per packet, HKDF-SHA256 expands (some seed ‖ counter) into an
AES-256-GCM key. The integrity commitment is HMAC-SHA256 (Encrypt-then-MAC over
the ciphertext). Standard arguments give:

- **IND-CPA** (confidentiality): authenticated encryption is IND-CPA secure
  under the AES hardness assumption and the uniform pseudorandomness of the
  HKDF output (ROM).
- **INT-CTXT** (integrity/authenticity): the HMAC-SHA256 EtM construction is
  unforgeable under the PRF security of HMAC-SHA256.
- Together, AEAD ⇒ **IND-CCA2**.

### A.2 Why the two can be composed end-to-end

The obstruction to a naive composition is that the BEE layer produces *shared
broadcast-capable* keys while the data layer uses *per-packet AEAD*. We close the
gap with an explicit key-domination chain:

```
msk ──BEE──▶ per-subset broadcast keys {K_S}
     ──HKDF──▶ K_root  = K_{S_full}  (the "all-active" subset key)
     ──HKDF──▶ K_session = HKDF(K_root, epoch)
     ──HKDF──▶ K_packet  = HKDF(K_session ‖ i)      (per-packet AEAD key)
     ──HMAC──▶ C_i = HMAC(K_packet, GCM(K_packet, m_i))
```

**Composition Theorem (sketch).** Let $\Pi_{\mathrm{BEE}}$ be
$(t,\varepsilon_{\mathrm{BEE}})$-collusion resistant (no coalition of size
$\le t$ learns any revoked key or $\mathsf{msk}$), and let the data layer
$(\mathsf{Gen},\mathsf{Enc},\mathsf{Dec})$ be $(\varepsilon_{\mathrm{AEAD}})$-
IND-CCA2 secure with keys drawn from a uniform 256-bit space. Then the composed
scheme is secure against an adversary $\mathcal{A}$ that:

- is a passive eavesdropper on all data ciphertexts, **and**
- controls a coalition of up to $t$ non-revoked, otherwise-honest swarm nodes.

against recovering the plaintext of any packet sent to a revoked node (or
forging a packet accepted by a legitimate receiver), except with advantage
bounded by
$\varepsilon_{\mathrm{BEE}} + q \cdot \varepsilon_{\mathrm{AEAD}}$
where $q$ is the total number of packets under the adversary's lease.

*Proof idea.* Consider the first hybrid where $\mathsf{msk}$ is replaced by a
uniform random string. By Component-1's collusion resistance this is
indistinguishable to the adversary (who by construction controls only
non-revoked leaves and hence never observes $\mathsf{msk}$ or any revoked leaf
key). In the second hybrid, replace each $K_{\mathrm{packet}}$ by uniform random
keys; by the ROM HKDF expansion and the uniformity of $K_{\mathrm{root}}$ this is
still indistinguishable. In the third hybrid replace the AEAD oracle by
$-perp$; by $\varepsilon_{\mathrm{AEAD}}$ IND-CCA2. Union-bounding over $q$ packet
challenges yields the stated bound. ∎

The crucially *separate* security of the two layers is what makes the
composition valid: the data-layer argument never requires the key-derivation to
hide *who* is revoked (that is entirely the BEE layer's job), and the BEE
argument never requires the transmission to be non-interceptable per packet (that
is the AEAD layer's job). This is the formal de-coupling that an end-to-end proof
requires, and it is precisely the property that a single-layer scheme (e.g. TLS
with per-pair symmetric keys) does *not* have — there, revocation and
confidentiality are entangled in the pairwise handshake, which is what makes
revocation `O(N)` expensive.

### A.3 The role of the chaotic pendulum (why not just a counter)

The data layer does **not** rely on the pendulum for either IND-CPA or INT-CTXT
— those hold for any uniform key source (this is exactly what the counter
baseline in Tier 1 demonstrates: it achieves identical AEAD goodput). The
pendulum's role is **entropy sourcing and forward-hiding**, a distinct property
not captured by the AEAD game alone:

> **Forward-hiding / sensitivity to state replay.** A single-bit corruption of
> the pendulum initial condition is amplified by the positive Lyapunov exponent
> $\lambda_1 \approx 1.48\ \text{nats}/\text{s}$ (time constant ≈ 0.68 s): the
> perturbed trajectory diverges from the reference by $O(1)$ in an average of
> $10.7\,\text{s}$ (≈ 16 Lyapunov time-scales; mean over all 256 bit positions,
> `results_v3/v3-corruption-test.json`), and the derived session key differs in
> **100%** of injected bit positions. In contrast, a counter-mode key source
> exposes the session seed directly: a single flipped bit changes only that
> bit's key and is detected trivially at epoch 0 with **no dynamic
> amplification** — an attacker who controls/corrupts the counter value can
> predict or replay keys that fail to hide prior state.

This is the honest reading of the corruption experiment: the counter detects a
corruption *faster* (immediately) but provides *no entropy amplification*,
whereas the pendulum provides dynamical amplification that is the actual
security property the design claims. Both detect an HMAC mismatch as soon as
they disagree; the meaningful difference is *why* the key stream becomes useless
and *at what guaranteed delay*. The protocol's §6.4 epoch bound is the
sampled-minimum entropy time-scale, $\max(256\ln2/\lambda_{\min},
1/\lambda_{\min})$, computed from the attractor-sampled $\lambda_{\min}\approx
0.75{-}0.80\ \text{nats}/\text{s}$ at the default Lyapunov window
($t=100\,\text{s}$), giving a bound ≈ 235 s and a ≈ 5× margin above the
1200 s epoch — with the caveat, already in the manuscript, that this is a
sampled minimum, not a proven global minimum. The divergence statistics above
(mean 10.7 s across 256 bit positions) bound the realistic breaking distance to
one `O(seconds-to-tens-of-seconds)` epoch.

---

## Part B — Dynamic Swarm Membership: Rebuild-Cost Bound

### B.1 The problem

The BEE tree is a *fixed* complete binary tree over $N$ leaves. A satellite that
joins post-deployment has no assigned leaf; the tree must be rebuilt. Naively
rebuilding and rebroadcasting the full covering set costs $O(N)$ keys per join,
which is exactly the cost the design tries to amortize away.

### B.2 A static worst-case bound on the per-join rebuild cost

Let the active swarm be size $N$, with $r$ revoked. A fresh satellite joins,
increasing active membership. Two quantities bound the per-join cost:

**Rebuild broadcast size.** Rebuilding the whole tree means re-deriving $N$
leaf keys and re-broadcasting enough of them that every (old) active node can
recover the new root $\mathsf{msk}'$ while revoked nodes remain excluded. With the
XOR-tree construction this requires the same transmission the covering set
already uses, i.e. one covering set of size
$$c(N,r) \;=\; \lceil \log_2 N / \log_2 r \rceil \cdot r \quad \text{(for } r\ge 2\text{)}.$$
So a full rebuild costs one more BEE broadcast of size $c(N,r) \cdot 64$ bytes
(32-byte key + 32-byte overhead), identical to a normal revocation. **In other
words: with this construction, a join costs no more than a revocation.**
Concretely at $N=1024, r=8$: $c = 4 \cdot 8 = 32$ blocks = $2048$ bytes — the
same figure the R-sweep uses.

**Amortized per-join lower bound.** Because the tree is balanced, a join only
*needs* to change the $\log_2 N$ keys on the new leaf's root-to-leaf path if we
are willing to tolerate a degraded (but functional) tree. The minimum number of
keys that must change is therefore
$$K_{\min} = \log_2 N,$$
which bounds the *key-derivation* cost; the *broadcast* cost is dominated by the
covering-set term above, not by $K_{\min}$.

### B.3 Extension: amortized joins with a lazy-rebuild schedule

If joins arrive at rate $\mu$ joins/s and we batch them into one rebuild every
$T$ seconds, the steady-state amortized per-join cost is

$$\frac{c(N, r+\Delta r(T)) \cdot 64 \ \text{bytes}}{\mu T} \;\to\; 0 \quad \text{as } T\to\infty,$$

but the *latency* of admitting a join grows linearly with $T$ (a joining node
cannot be admitted until the next rebuild). This is the classic
**amortized-cost vs admission-latency** trade-off, exactly analogous to the
commitment-interval trade-off in Tier 2c. The design point is to pick $T$ so the
amortized broadcast stays a small fraction of the downlink capacity.

### B.4 Concrete worked example (admission latency vs rebuild overhead)

Using the simulator's measured numbers ($N=1024$, payload 1024 B, measured
goodput ≈ 1.65 Mbps ⇒ ≈ 247 MB per 1200 s epoch, mean one-way link latency
≈ 4.77 ms):

| Join rate | Rebuild interval $T$ | Amortized per-join BEE bytes (at r=8) | Admission latency | Rebuild goodput impact |
|---|---|---|---|---|
| 1 / epoch | immediate | 2048 B | 1 epoch | 0.0008% of epoch capacity |
| 1 / epoch | 10 epochs (3.33 h) | 204.8 B | ≤ 10 epochs | 0.00008% |
| 1 / 60 s | 6 epochs (2 h) | ~341 B | ≤ 6 epochs | ~0.00014% |
| 1 / 10 s | 60 s | ~3413 B | ≤ 60 s | ~0.0005% of capacity |

The last row shows a realistic worst case: even admitting a satellite every 10
seconds with a 60-second rebuild window only consumes ~0.0005% of epoch
downlink capacity. Even compared against the *smallest* data overhead in the
swcep (the 1024 B payload, whose own crypto/message overhead is 3.125%), BEE
rebuild broadcasts are 3–4 orders of magnitude smaller. This converts the
"dynamic membership is an open problem" prose into a concrete, bounded cost
model grounded in the measured goodput.

---

## Conclusion

- **Part A** gives the first explicit end-to-end composition of the BEE
  collusion-resistance argument with the data-layer AEAD argument, with a
  concrete hybrid-argument advantage bound, and clarifies (with measured data)
  exactly what the chaotic pendulum contributes that a counter does not.
- **Part B** bounds the dynamic-membership rebuild cost, showing a join costs no
  more than a revocation with this XOR-tree construction, and gives a worked
  amortized-vs-latency schedule with realistic simulator parameters.
