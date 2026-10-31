"""fixweights.py: re-skin a Tripo auto-rigged (Mixamo-named) GLB so cloth and hair follow the body,
not the hands. Only JOINTS_0/WEIGHTS_0 bytes change; mesh, UVs, material, JPEG, armature, rest pose
and clips stay byte-identical (see glbio.save_weights).

Method (positions in glTF units; Tripo normalises characters to 1.0 tall):
 1. Limb locators. Each arm/leg is a set of bone segments with a tube radius. If a limb's bones sit
    far from its mesh (Tripo sometimes puts a whole arm chain in the cape), the mirror of the other
    side's chain is used to find that limb's mesh instead ("misplaced" in the report).
 2. Limb sets, decided per connected island (vertices welded across UV seams):
      island mostly inside the tube        -> whole island belongs to the limb (cuffs, bracelets);
      island with a real core in the tube  -> fused piece (sleeve welded to a cape): only the tube
                                              vertices belong to the limb;
      otherwise                            -> nothing (hair strands, cape panels passing an arm).
 3. Limb weights: the original Tripo weights, restricted to that limb's chain plus its root bones.
    A misplaced limb instead copies the mirrored vertex weights of the good side (L<->R swap).
 4. Everything else gets procedural trunk weights: inverse-distance (p=4) to the spine, neck, head
    and shoulder segments, plus a thigh share for cloth below the hips that fades out with distance
    from the thighs (skirts follow the legs a little, a long cape mostly follows the hips).
 5. Smooth seam: around each limb set, geodesic distance over the welded mesh gives a falloff band
    (smoothstep over BAND) that blends the limb weights of the nearest set vertex into the trunk
    weights, so a cape welded to a sleeve stretches a little instead of tearing.
 6. Top 4 influences, normalised; no vertex left without weight.

It refuses a mesh that is one continuous island (exit code 3, report says "one_island"): a limb tube
cannot tell a sleeve from a body there and the seam band tears sleeves and legs. Use narrow.py instead.

Usage: blender -b --factory-startup -P fixweights.py -- in.glb out.glb [--debug prefix] [--band metres]
           [--report-only]
       (or Blender's own python: <python> -I fixweights.py in.glb out.glb; it needs numpy only)
--report-only writes out.glb.report.json (limbs, misplaced chains) and no GLB. --band widens the seam.
All distances are for a model one unit tall and scale with the model's real height.
"""
import sys, os, json, heapq
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np, glbio

# ---- tunables (unit-height character) ----
R_UPPERARM, R_FOREARM, R_HAND = 0.030, 0.025, 0.022
R_THIGH, R_SHIN, R_FOOT = 0.055, 0.032, 0.040
FIT_RATIO = 2.0            # mirrored chain must hug this many times more verts to call a limb misplaced
ISLAND_INSIDE = 0.6        # island fraction inside the tube to give the whole island to the limb
CORE_FRAC, CORE_MIN = 0.6, 30
MIRROR_TOL = 0.005         # a mirrored limb keeps only verts whose mirror lies this close to the good limb
DANGLE_MAX = 400            # welded verts; pieces cut off by a limb set up to this size join the limb
BAND = 0.05                # geodesic falloff width outside a limb set
LEG_NEAR, LEG_FAR, LEG_MAX = 0.05, 0.16, 0.6
P_IDW = 4.0


def seg_dist(P, a, b):
    ab = b - a
    t = np.clip(((P - a) @ ab) / max(ab @ ab, 1e-12), 0, 1)
    return np.linalg.norm(P - (a + t[:, None] * ab), axis=1)


def smoothstep(x):
    x = np.clip(x, 0, 1)
    return x * x * (3 - 2 * x)


def scale_tunables(h):
    """The tube radii and bands are for a model one unit tall."""
    global R_UPPERARM, R_FOREARM, R_HAND, R_THIGH, R_SHIN, R_FOOT, MIRROR_TOL, BAND, LEG_NEAR, LEG_FAR
    R_UPPERARM, R_FOREARM, R_HAND = R_UPPERARM * h, R_FOREARM * h, R_HAND * h
    R_THIGH, R_SHIN, R_FOOT = R_THIGH * h, R_SHIN * h, R_FOOT * h
    MIRROR_TOL, BAND, LEG_NEAR, LEG_FAR = MIRROR_TOL * h, BAND * h, LEG_NEAR * h, LEG_FAR * h


def main(src, dst, debug=None, band=None, report_only=False):
    global BAND
    if band is not None:
        BAND = band
    g = glbio.load(src)
    scale_tunables(glbio.height(g))
    V, NJ = len(g.pos), len(g.names)
    hips = next(n for n in g.names if n.endswith("Hips"))
    PFX = hips[:-4]
    ni = {n: i for i, n in enumerate(g.names)}
    J = lambda n: ni[PFX + n]
    JP = lambda n: g.jpos[J(n)]
    M = np.array([-1.0, 1.0, 1.0])
    W0 = glbio.dense(g)
    lab, inv = glbio.components(g)
    if glbio.one_island(lab):
        json.dump({"one_island": True, "limbs": {}}, open(dst + ".report.json", "w"), indent=1)
        print("REFUSED one_island: one connected island holds nearly every vertex, so a limb tube would tear sleeves and legs. Use narrow.py.")
        sys.exit(3)
    ncomp = lab.max() + 1
    csize = np.bincount(lab)

    # welded graph (for geodesics)
    nw = inv.max() + 1
    wp = np.zeros((nw, 3)); wp[inv] = g.pos
    t = inv[g.idx]
    e = np.concatenate([t[:, [0, 1]], t[:, [1, 2]], t[:, [2, 0]]])
    e = e[e[:, 0] != e[:, 1]]
    e = np.unique(np.sort(e, 1), axis=0)
    el = np.linalg.norm(wp[e[:, 0]] - wp[e[:, 1]], axis=1)
    adj = [[] for _ in range(nw)]
    for (u, v), l in zip(e.tolist(), el.tolist()):
        adj[u].append((v, l)); adj[v].append((u, l))

    fingers = ["Thumb", "Index", "Middle", "Ring", "Pinky"]
    def limb_segments(side, kind, mirror=False):
        o = side if not mirror else ("Right" if side == "Left" else "Left")
        P = (lambda n: JP(o + n) * M) if mirror else (lambda n: JP(o + n))
        if kind == "arm":
            segs = [(P("Arm"), P("ForeArm"), R_UPPERARM), (P("ForeArm"), P("Hand"), R_FOREARM)]
            for f in fingers:
                if PFX + o + "Hand" + f + "4" in ni:
                    segs.append((P("Hand"), P("Hand" + f + "1"), R_HAND))
                    segs.append((P("Hand" + f + "1"), P("Hand" + f + "4"), R_HAND))
        else:
            segs = [(P("UpLeg"), P("Leg"), R_THIGH), (P("Leg"), P("Foot"), R_SHIN),
                    (P("Foot"), P("ToeBase"), R_FOOT), (P("ToeBase"), P("Toe_End"), R_FOOT)]
        return segs

    def ndist(segs):
        return np.min([seg_dist(g.pos, a, b) / r for a, b, r in segs], axis=0)

    def chain(side, kind):
        if kind == "arm":
            base = ["Shoulder", "Arm", "ForeArm", "Hand"] + [f"Hand{f}{k}" for f in fingers for k in range(1, 5)]
            roots = ["Spine2", "Spine1", "Neck"]
            return [J(side + n) for n in base if PFX + side + n in ni], [J(n) for n in roots]
        base = ["UpLeg", "Leg", "Foot", "ToeBase", "Toe_End"]
        return [J(side + n) for n in base if PFX + side + n in ni], [J("Hips"), J("Spine")]

    limbs = []
    report = {"limbs": {}}
    for kind in ("arm", "leg"):
        for side in ("Left", "Right"):
            own = ndist(limb_segments(side, kind))
            mir = ndist(limb_segments(side, kind, mirror=True))
            # fit test on the distal part (hand / foot) where a misplaced chain shows most
            dsegs = slice(2, None) if kind == "arm" else slice(1, None)
            own_fit = (np.min([seg_dist(g.pos, a, b) / r for a, b, r in limb_segments(side, kind)[dsegs]], axis=0) < 0.6).sum()
            mir_fit = (np.min([seg_dist(g.pos, a, b) / r for a, b, r in limb_segments(side, kind, True)[dsegs]], axis=0) < 0.6).sum()
            misplaced = mir_fit > FIT_RATIO * own_fit and mir_fit > 50
            d = mir if misplaced else own
            limbs.append(dict(kind=kind, side=side, d=d, misplaced=misplaced))
            report["limbs"][side + kind] = dict(own_fit=int(own_fit), mirrored_fit=int(mir_fit), misplaced=bool(misplaced))

    # ---- limb sets per island ----
    for L in limbs:
        tube = L["d"] < 1.0
        core = L["d"] < CORE_FRAC
        nt = np.bincount(lab, weights=tube, minlength=ncomp)
        nc = np.bincount(lab, weights=core, minlength=ncomp)
        whole = nt / csize >= ISLAND_INSIDE
        fused = (nc >= CORE_MIN) & ~whole
        # fused pieces: keep only tube regions connected (inside the tube) to the limb core,
        # so a cape edge or skirt panel touching the tube far from the limb is not taken
        tw = np.zeros(nw, bool); tw[inv[tube & fused[lab]]] = True
        cw_ = np.zeros(nw, int); np.add.at(cw_, inv[core & fused[lab]], 1)
        ee = e[tw[e[:, 0]] & tw[e[:, 1]]]
        par = np.arange(nw)
        def find(x):
            while par[x] != x:
                par[x] = par[par[x]]; x = par[x]
            return x
        for a_, b_ in ee.tolist():
            ra, rb = find(a_), find(b_)
            if ra != rb: par[ra] = rb
        roots = np.array([find(x) for x in range(nw)])
        corecount = np.bincount(roots, weights=cw_, minlength=nw)
        keepw = tw & (corecount[roots] >= CORE_MIN)
        L["set"] = whole[lab] | keepw[inv]
        L["fused"] = fused[lab]
        if debug:
            for c in np.where(fused)[0]:
                m = lab == c
                print("FUSED", L["side"], L["kind"], "comp", c, "size", m.sum(), "tube", int(nt[c]), "core", int(nc[c]), "ybox", g.pos[m][:, 1].min().round(2), g.pos[m][:, 1].max().round(2))
    # misplaced limbs: register the mirrored good limb (its set is final here, good limbs need no change)
    for L in limbs:
        if not L["misplaced"]:
            continue
        other = next(o for o in limbs if o["kind"] == L["kind"] and o["side"] != L["side"])
        # Register the mirrored good limb onto this side's mesh, one rigid trimmed-ICP fit per
        # bone segment (the two arms are never posed exactly alike), then take as this limb's
        # set only the verts lying on the registered surface. Cape strips welded right behind
        # the arm are not on the mirrored arm surface, so they stay with the torso.
        src_idx = np.where(other["set"])[0]
        pool = np.where(L["d"] < 1.6)[0]
        poolp = g.pos[pool]
        segs = limb_segments(L["side"], L["kind"], mirror=True)
        core_segs = segs[:2] + [(segs[1][1], np.mean([b_ for _, b_, _ in segs[2:]], 0), R_HAND)] if L["kind"] == "arm" else segs
        srcp0 = g.pos[src_idx] * M
        grp = np.argmin(np.stack([seg_dist(srcp0, a_, b_) for a_, b_, _ in core_segs], 1), 1)
        srcp = srcp0.copy()
        for k_ in range(len(core_segs)):
            m_ = grp == k_
            if m_.sum() < 10:
                continue
            P_ = srcp0[m_]
            Rm, tv = np.eye(3), np.zeros(3)
            for it in range(30):
                Q = P_ @ Rm.T + tv
                nn = np.empty(len(Q), int); nd = np.empty(len(Q))
                for i0 in range(0, len(Q), 1000):
                    dd = ((Q[i0:i0 + 1000, None, :] - poolp[None]) ** 2).sum(-1)
                    nn[i0:i0 + 1000] = dd.argmin(1); nd[i0:i0 + 1000] = dd.min(1)
                keep_ = nd <= np.quantile(nd, 0.7)
                A_, B_ = Q[keep_], poolp[nn[keep_]]
                ca, cb = A_.mean(0), B_.mean(0)
                U, _, Vt = np.linalg.svd((A_ - ca).T @ (B_ - cb))
                Rs = (U @ Vt).T
                if np.linalg.det(Rs) < 0:
                    Vt[-1] *= -1; Rs = (U @ Vt).T
                Rm = Rs @ Rm; tv = Rs @ (tv - ca) + cb
            srcp[m_] = P_ @ Rm.T + tv
            L.setdefault("xf", {})[k_] = (Rm.copy(), tv.copy())
            report["limbs"][L["side"] + L["kind"]].setdefault("icp_shift_cm", []).append(round(float(np.linalg.norm(srcp[m_] - P_, axis=1).mean()) * 100, 2))
        # dense samples of the registered surface
        pos_reg = np.zeros_like(g.pos); pos_reg[src_idx] = srcp
        tri = g.idx[other["set"][g.idx].all(1)]
        bc = np.array([(i / 4, j / 4, 1 - i / 4 - j / 4) for i in range(5) for j in range(5 - i)])
        samp = np.einsum("sk,tkd->tsd", bc, pos_reg[tri]).reshape(-1, 3)
        sd = np.empty(len(pool))
        for i0 in range(0, len(pool), 500):
            sd[i0:i0 + 500] = np.sqrt(((poolp[i0:i0 + 500, None, :] - samp[None]) ** 2).sum(-1).min(1))
        if debug: print('SDHIST', np.histogram(sd, bins=[0,.002,.004,.006,.008,.01,.015,.02,.03,.05,1])[0].tolist())
        onsurf = np.zeros(V, bool); onsurf[pool[sd < MIRROR_TOL]] = True
        # islands (cuffs, bracelets) that mostly lie on the registered surface belong to the limb whole
        near_ = np.zeros(V, bool); near_[pool[sd < 2 * MIRROR_TOL]] = True
        whole2 = np.bincount(lab, weights=near_, minlength=ncomp) / csize >= ISLAND_INSIDE
        L["set"] = L["set"] | (whole2[lab] & ~L["fused"])
        L["reg_groups"] = [(k_, grp == k_) for k_ in range(len(core_segs))]
        # whole islands already given to this limb stay; on fused pieces keep only on-surface verts
        L["set"] = (L["set"] & ~L["fused"]) | (onsurf & (L["fused"] | L["set"]))
        L["src_idx"], L["srcp"] = src_idx, srcp
        # where this limb's joints would have to be to match its mesh (used by rigmove.py and probe.py)
        o = other["side"]
        def reg(n, groups):
            p = JP(o + n) * M
            return np.mean([L["xf"][k_][0] @ p + L["xf"][k_][1] for k_ in groups if k_ in L.get("xf", {})] or [p], 0)
        if L["kind"] == "arm":
            names = {"Arm": [0], "ForeArm": [0, 1], "Hand": [1, 2]}
            names.update({f"Hand{f}{k}": [2] for f in fingers for k in range(1, 5) if PFX + o + f"Hand{f}{k}" in ni})
        else:
            names = {"UpLeg": [0], "Leg": [0, 1], "Foot": [1, 2], "ToeBase": [2, 3], "Toe_End": [3]}
        report["limbs"][L["side"] + L["kind"]]["registered_joints"] = {PFX + L["side"] + n: [round(float(x), 5) for x in reg(n, gr)] for n, gr in names.items()}
    # dangling bits: pieces of a fused island that are cut off by a limb set (a ribbon or chain
    # welded to a glove) are small; they hang from that limb, so they join it
    for L in limbs:
        inset_w = np.zeros(nw, bool); inset_w[inv[L["set"]]] = True
        fz = np.zeros(nw, bool); fz[inv[L["fused"]]] = True
        free = fz & ~inset_w
        par = np.arange(nw)
        def find2(x):
            while par[x] != x:
                par[x] = par[par[x]]; x = par[x]
            return x
        for a_, b_ in e[free[e[:, 0]] & free[e[:, 1]]].tolist():
            ra, rb = find2(a_), find2(b_)
            if ra != rb: par[ra] = rb
        roots = np.array([find2(x) for x in range(nw)])
        sz = np.bincount(roots[free], minlength=nw)
        touch = np.zeros(nw, bool)
        ed = e[(free[e[:, 0]] & inset_w[e[:, 1]]) | (free[e[:, 1]] & inset_w[e[:, 0]])]
        for a_, b_ in ed.tolist():
            touch[roots[a_ if free[a_] else b_]] = True
        small = free & (sz[roots] <= DANGLE_MAX) & touch[roots]
        L["set"] = L["set"] | small[inv]
        report["limbs"][L["side"] + L["kind"]]["dangling_joined"] = int(small[inv].sum())
    # resolve overlaps: nearest limb (normalised distance) wins
    D = np.stack([np.where(L["set"], L["d"], np.inf) for L in limbs])
    win = np.argmin(D, 0)
    for k, L in enumerate(limbs):
        L["set"] = L["set"] & (win == k)

    fusedmask = [L["fused"] for L in limbs]
    # ---- limb weights ----
    Wlimb = np.zeros((V, NJ))
    order = sorted(range(4), key=lambda k: limbs[k]["misplaced"])  # good limbs first, mirrored ones copy them
    for k in order:
        L = limbs[k]
        s = np.where(L["set"])[0]
        base, roots = chain(L["side"], L["kind"])
        allowed = base + roots
        if not L["misplaced"]:
            w = W0[s][:, allowed]
            mass = w.sum(1)
            # vertices whose originals are mostly elsewhere: nearest limb-chain bone fallback
            bad = mass < 0.3
            if bad.any():
                segs = []
                for b in base:
                    kids = [c for c, p in enumerate(g.parent) if p == b]
                    if kids:
                        segs.append((b, g.jpos[b], g.jpos[kids[0]]))
                dd = np.stack([seg_dist(g.pos[s[bad]], a, c) for _, a, c in segs], 1)
                iw = 1.0 / np.maximum(dd, 0.005) ** P_IDW
                fb = np.zeros((bad.sum(), NJ))
                for col, (b, _, _) in enumerate(segs):
                    fb[:, b] += iw[:, col]
                w_full = np.zeros((len(s), NJ)); w_full[:, allowed] = w
                w_full[bad] = fb
                w_full /= w_full.sum(1, keepdims=True)
                Wlimb[s] = w_full
            else:
                w_full = np.zeros((len(s), NJ)); w_full[:, allowed] = w / mass[:, None]
                Wlimb[s] = w_full
        else:
            src_idx, srcp = L["src_idx"], L["srcp"]
            s = np.where(L["set"])[0]
            near = np.empty(len(s), int)
            for i0 in range(0, len(s), 1000):
                dd = ((g.pos[s[i0:i0 + 1000], None, :] - srcp[None]) ** 2).sum(-1)
                near[i0:i0 + 1000] = src_idx[dd.argmin(1)]
            report["limbs"][L["side"] + L["kind"]]["set_after_registration"] = int(len(s))
            # swap Left<->Right bone columns
            swap = np.arange(NJ)
            for i, n in enumerate(g.names):
                for a, b in (("Left", "Right"), ("Right", "Left")):
                    if n.startswith(PFX + a) and PFX + b + n[len(PFX + a):] in ni:
                        swap[i] = ni[PFX + b + n[len(PFX + a):]]
            Wsrc = Wlimb[near]
            Wm = np.zeros_like(Wsrc)
            Wm[:, swap] = Wsrc
            Wlimb[s] = Wm

    # ---- trunk weights (procedural) ----
    tsegs = [("Hips", "Spine"), ("Spine", "Spine1"), ("Spine1", "Spine2"), ("Spine2", "Neck"), ("Neck", "Head"),
             ("Head", "HeadTop_End"), ("LeftShoulder", "LeftArm"), ("RightShoulder", "RightArm")]
    dd = np.stack([seg_dist(g.pos, JP(a), JP(b)) for a, b in tsegs], 1)
    iw = 1.0 / np.maximum(dd, 0.01) ** P_IDW
    Wtr = np.zeros((V, NJ))
    for col, (a, _) in enumerate(tsegs):
        Wtr[:, J(a)] += iw[:, col]
    Wtr /= Wtr.sum(1, keepdims=True)
    # thigh share for cloth below the hips
    th = [("LeftUpLeg", "LeftLeg"), ("RightUpLeg", "RightLeg")]
    dl = np.stack([seg_dist(g.pos, JP(a), JP(b)) for a, b in th], 1)
    lw = 1.0 / np.maximum(dl, 0.01) ** P_IDW
    lw /= lw.sum(1, keepdims=True)
    hip_y = JP("Hips")[1]
    below = smoothstep((hip_y - g.pos[:, 1]) / 0.05)
    u = LEG_MAX * smoothstep((LEG_FAR - dl.min(1)) / (LEG_FAR - LEG_NEAR)) * below
    Wleg = np.zeros((V, NJ))
    Wleg[:, J("LeftUpLeg")] = lw[:, 0]; Wleg[:, J("RightUpLeg")] = lw[:, 1]
    Wtrunk = (1 - u)[:, None] * Wtr + u[:, None] * Wleg

    # ---- falloff band around limb sets (geodesic, nearest set vertex carries the limb weights) ----
    A = np.zeros((len(limbs), V))
    Wband = np.zeros((len(limbs), V, NJ))
    for k, L in enumerate(limbs):
        s = np.where(L["set"])[0]
        geo = np.full(nw, np.inf); src = np.full(nw, -1)
        hq = []
        for vi in s:
            w_ = inv[vi]
            if geo[w_] > 0:
                geo[w_] = 0.0; src[w_] = vi; hq.append((0.0, w_))
        heapq.heapify(hq)
        while hq:
            d0, u_ = heapq.heappop(hq)
            if d0 > geo[u_]:
                continue
            for v_, l in adj[u_]:
                nd = d0 + l
                if nd < geo[v_] and nd < BAND:
                    geo[v_] = nd; src[v_] = src[u_]; heapq.heappush(hq, (nd, v_))
        gv = geo[inv]; sv = src[inv]
        a = np.where(L["set"], 1.0, smoothstep(1 - gv / BAND))
        a[~np.isfinite(gv)] = 0
        A[k] = a
        has = sv >= 0
        Wband[k][has] = Wlimb[sv[has]]
        Wband[k][L["set"]] = Wlimb[L["set"]]
    asum = A.sum(0)
    scale = np.where(asum > 1, 1 / np.maximum(asum, 1e-9), 1.0)
    A *= scale
    W = (1 - A.sum(0))[:, None] * Wtrunk + np.einsum("kv,kvj->vj", A, Wband)

    joints, weights = glbio.to_top4(W)
    if not report_only:
        glbio.save_weights(g, joints, weights, dst)

    Wn = np.zeros((V, NJ)); np.add.at(Wn, (np.repeat(np.arange(V), 4), joints.astype(int).ravel()), weights.ravel())
    changed = np.abs(Wn - W0).sum(1) > 0.05
    report.update(verts=V, tris=len(g.idx), changed=int(changed.sum()),
                  limb_set_sizes={L["side"] + L["kind"]: int(L["set"].sum()) for L in limbs},
                  band_verts=int(((A.sum(0) > 0) & (A.sum(0) < 1)).sum()))
    print("REPORT", json.dumps({k: v for k, v in report.items()}, default=str)[:2000])
    json.dump(report, open(dst + ".report.json", "w"), indent=1, default=str)
    if debug:
        cols = np.array([(1, .2, .2), (.2, .4, 1), (1, .9, 0), (0, .9, .9)])
        c = np.full((V, 3), 0.45)
        for k in range(4):
            c = c * (1 - A[k][:, None]) + A[k][:, None] * cols[k]
        np.save(debug + "_sets.npy", c)


if __name__ == "__main__":
    a = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    dbg = a[a.index("--debug") + 1] if "--debug" in a else None
    bnd = float(a[a.index("--band") + 1]) if "--band" in a else None
    main(a[0], a[1], dbg, bnd, "--report-only" in a)
