#!/usr/bin/env python3
"""Patch std's `unsupported::Instant::now`/`SystemTime::now` in a wasm binary
to tail-call web_time's implementations.

Why: gpui-pre's frame instrumentation (and a few gpui-base/component paths)
call `std::time::Instant::now`, which panics on wasm32-unknown-unknown. The
crates can't be edited, but every caller routes through the single
`sys::time::unsupported` function — which shares its ABI with
`web_time::{Instant,SystemTime}` (both are `X(Duration)` returned via the
same sret/multivalue convention). We find both functions by their names in
the name section and replace the panicking body with a `call` to the
web_time twin. Idempotent; run between `cargo build` and `wasm-bindgen`.
"""
import sys


def leb(d, i):
    r = s = 0
    while True:
        b = d[i]
        i += 1
        r |= (b & 0x7F) << s
        if not b & 0x80:
            return r, i
        s += 7


def leb_enc(v):
    out = bytearray()
    while True:
        b = v & 0x7F
        v >>= 7
        if v:
            out.append(b | 0x80)
        else:
            out.append(b)
            return bytes(out)


def sections(d):
    i, out = 8, []
    while i < len(d):
        sid = d[i]
        i += 1
        size, j = leb(d, i)
        out.append((sid, j, j + size))
        i = j + size
    return out


def type_sigs(d, secs):
    """func type index -> list of (params, results) valtype counts."""
    for sid, a, b in secs:
        if sid != 1:
            continue
        sigs = []
        j = a
        n, j = leb(d, j)
        for _ in range(n):
            assert d[j] == 0x60
            j += 1
            np, j = leb(d, j)
            params = d[j : j + np]
            j += np
            nr, j = leb(d, j)
            results = d[j : j + nr]
            j += nr
            sigs.append((params, results))
        return sigs
    return []


def imported_funcs(d, secs):
    """number of imported functions (they precede defined ones)."""
    for sid, a, b in secs:
        if sid != 2:
            continue
        j = a
        n, j = leb(d, j)
        count = 0
        for _ in range(n):
            ml, j = leb(d, j)
            j += ml
            nl, j = leb(d, j)
            j += nl
            kind = d[j]
            j += 1
            if kind == 0:
                j += 1  # typeidx
                count += 1
            elif kind == 1:  # table
                j += 1
                lim = d[j]
                j += 1
                _, j = leb(d, j)
                if lim & 1:
                    _, j = leb(d, j)
            elif kind == 2:  # memory
                fl, j = leb(d, j)
                _, j = leb(d, j)
                if fl & 1:
                    _, j = leb(d, j)
            elif kind == 3:  # global
                j += 2
            elif kind == 4:  # tag
                j += 1
                _, j = leb(d, j)
        return count
    return 0


def func_types(d, secs):
    for sid, a, b in secs:
        if sid != 3:
            continue
        j = a
        n, j = leb(d, j)
        out = []
        for _ in range(n):
            v, j = leb(d, j)
            out.append(v)
        return out
    return []


def func_names(d, secs):
    """funcidx -> name from the name section's function-name subsection."""
    for sid, a, b in secs:
        if sid != 0 or d[a : a + 5] != b"\x04name":
            continue
        j = a + 5
        names = {}
        while j < b:
            sub = d[j]
            j += 1
            size, j = leb(d, j)
            end = j + size
            if sub == 1:
                k = j
                n, k = leb(d, k)
                for _ in range(n):
                    idx, k = leb(d, k)
                    ln, k = leb(d, k)
                    names[idx] = d[k : k + ln].decode("utf-8", "replace")
                    k += ln
            j = end
        return names
    return {}


def find(names, *needles):
    hits = [
        i for i, n in names.items() if all(needle in n for needle in needles)
    ]
    if len(hits) != 1:
        sys.exit(f"{needles}: expected 1 match, got {hits}")
    return hits[0]


def patch(path):
    d = bytearray(open(path, "rb").read())
    secs = sections(d)
    sigs = type_sigs(d, secs)
    n_imported = imported_funcs(d, secs)
    ftypes = func_types(d, secs)
    names = func_names(d, secs)

    # (module-marker, type, method) — the public `std::time::X::now` may
    # carry an inlined copy of the panic, and the `sys::unsupported` copy
    # may also exist; patch whichever is present.
    pairs = [
        (False, "Instant", "now"),
        (False, "SystemTime", "now"),
        (True, "Instant", "now"),
        (True, "SystemTime", "now"),
    ]
    # sources may be absent entirely when dead-code-eliminated; dst
    # (web_time's twin) is required for Instant and optional for the rest.
    if not names:
        sys.exit("no function-name subsection — build may strip names?")
    # name-section entries look like
    # `_RNvMNtNtNtCs.._3std3sys4time11unsupportedNtB2_7Instant3now`
    rewrites = []
    for is_sys, ty, meth in pairs:
        src_hits = [
            i
            for i, n in names.items()
            if "3std" in n
            and "4time" in n
            and f"{len(ty)}{ty}3{meth}" in n
            and ("unsupported" in n) == is_sys
        ]
        if not src_hits:
            print(f"{ty}::now (sys={is_sys}): absent — skipping")
            continue
        if len(src_hits) != 1:
            sys.exit(f"{ty}::now (sys={is_sys}): {src_hits}")
        src = src_hits[0]
        dst_hits = [
            i
            for i, n in names.items()
            if "web_time" in n and f"{len(ty)}{ty}3{meth}" in n
        ]
        if len(dst_hits) == 1:
            dst = dst_hits[0]
            if ftypes[src - n_imported] != ftypes[dst - n_imported]:
                sys.exit(
                    f"signature mismatch for {ty}::now: "
                    f"{sigs[ftypes[src - n_imported]]} vs "
                    f"{sigs[ftypes[dst - n_imported]]}"
                )
            rewrites.append((src, dst))
            print(f"{ty}::now: func[{src}] -> call func[{dst}]")
        else:
            # web_time's twin was dead-code-eliminated — emit a constant
            # epoch value instead (signature decides sret vs multivalue).
            rewrites.append((src, None))
            print(f"{ty}::now: func[{src}] -> constant epoch")

    # code section: vec of bodies for defined functions
    for sid, a, b in secs:
        if sid != 10:
            continue
        j = a
        n, j = leb(d, j)
        bodies = []
        for _ in range(n):
            sz, k = leb(d, j)
            bodies.append(d[k : k + sz])
            j = k + sz
        patched = set()
        for src, dst in rewrites:
            params, results = sigs[ftypes[src - n_imported]]
            body = bytearray(b"\x00")  # no locals
            if dst is not None:
                for i in range(len(params)):
                    body += b"\x20" + leb_enc(i)  # local.get i
                body += b"\x10" + leb_enc(dst)  # call dst
            elif results:
                # multivalue return — push a zero per result type
                for t in results:
                    body += {0x7F: b"\x41\x00", 0x7E: b"\x42\x00"}.get(
                        t, b"\x41\x00"
                    )
            else:
                # sret out-pointer — zero a 16-byte Duration struct
                for off in (0, 8):
                    body += b"\x20\x00\x42\x00\x37\x03" + bytes([off])
            body += b"\x0b"
            bodies[src - n_imported] = bytes(body)
            patched.add(src)

        # LLVM treated the original panic-only `now` as noreturn, so call
        # sites carry `call <now>; unreachable`. Now that it returns, the
        # unreachable traps — same-size swap it for a nop.
        nops = 0
        for i, body in enumerate(bodies):
            for src in patched:
                pat = b"\x10" + leb_enc(src) + b"\x00"
                if pat in body:
                    bodies[i] = body = body.replace(pat, pat[:-1] + b"\x01")
                    nops += 1
        if nops:
            print(f"call-site noreturn traps neutralized: {nops}")

        # re-emit: copy every section verbatim except the code section
        out = bytearray(d[:8])
        for sid2, a2, b2 in secs:
            if sid2 == 10:
                payload = leb_enc(n)
                for body in bodies:
                    payload += leb_enc(len(body)) + body
                out += bytes([10]) + leb_enc(len(payload)) + payload
            else:
                # walk back over the size LEB to the section-id byte:
                # a2-1 is the LEB's last byte; earlier bytes carry the
                # continuation bit.
                p = a2 - 1
                while d[p - 1] & 0x80:
                    p -= 1
                p -= 1
                out += d[p:b2]
        open(path, "wb").write(out)
        return
    sys.exit("no code section found")


if __name__ == "__main__":
    patch(sys.argv[1])
