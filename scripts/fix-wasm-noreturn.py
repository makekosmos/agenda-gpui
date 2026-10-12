#!/usr/bin/env python3
"""Post-wasm-bindgen pass: neutralize `unreachable` after calls to the
now-stubs injected by patch-wasm-time.py.

Why: LLVM marked the panic-only `std::time::*::now` functions noreturn, so
call sites end `call <now>; unreachable`. patch-wasm-time makes `now`
return — the stale `unreachable` then traps. wasm-bindgen's threading
transform rewrites function indices, so this pattern only becomes visible
after bindgen; hence a second byte-exact pass on `_bg.wasm`: find the stub
bodies by their shape (`no locals; local.get*; call; end`), then flip the
`unreachable` after calls to them into `nop`. Same-size edit, in place.
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


def enc(v):
    out = bytearray()
    while True:
        b = v & 0x7F
        v >>= 7
        if v:
            out.append(b | 0x80)
        else:
            out.append(b)
            return bytes(out)


def is_stub(body):
    """00 (20 <idx>)* 10 <funcidx> 0b — push args, tail-call, return."""
    if not (body.startswith(b"\x00") and body.endswith(b"\x0b")):
        return False
    p = 1
    while p < len(body) - 1 and body[p] == 0x20:
        _, p = leb(body, p + 1)
    if p >= len(body) - 1 or body[p] != 0x10:
        return False
    _, p = leb(body, p + 1)
    return p == len(body) - 1


def main(path):
    d = bytearray(open(path, "rb").read())
    i, secs = 8, []
    while i < len(d):
        sid = d[i]
        i += 1
        size, j = leb(d, i)
        secs.append((sid, j, j + size))
        i = j + size

    imported = 0
    for sid, a, b in secs:
        if sid != 2:
            continue
        j = a
        n, j = leb(d, j)
        for _ in range(n):
            ml, j = leb(d, j)
            j += ml
            nl, j = leb(d, j)
            j += nl
            kind = d[j]
            j += 1
            if kind == 0:
                _, j = leb(d, j)
                imported += 1
            elif kind == 1:
                j += 1
                lim = d[j]
                j += 1
                _, j = leb(d, j)
                if lim & 1:
                    _, j = leb(d, j)
            elif kind == 2:
                fl, j = leb(d, j)
                _, j = leb(d, j)
                if fl & 1:
                    _, j = leb(d, j)
            elif kind == 3:
                j += 2
            elif kind == 4:
                j += 1
                _, j = leb(d, j)

    for sid, a, b in secs:
        if sid != 10:
            continue
        j = a
        n, j = leb(d, j)
        bodies = []
        for _ in range(n):
            sz, k = leb(d, j)
            bodies.append((k, k + sz))
            j = k + sz

        stubs = {
            idx
            for idx, (k0, k1) in enumerate(bodies, start=imported)
            if is_stub(bytes(d[k0:k1]))
        }
        if not stubs:
            print("no now-stubs found — nothing to fix")
            return

        fixed = []
        for idx, (k0, k1) in enumerate(bodies, start=imported):
            body = bytes(d[k0:k1])
            new = body
            for s in stubs:
                pat = b"\x10" + enc(s) + b"\x00"  # call s; unreachable
                if pat in new:
                    new = new.replace(pat, pat[:-1] + b"\x01")  # -> nop
            if new != body:
                d[k0:k1] = new
                fixed.append(idx)
        print(f"now-stubs: {sorted(stubs)}; fixed call sites: {fixed}")
        open(path, "wb").write(d)
        return
    sys.exit("no code section found")


if __name__ == "__main__":
    main(sys.argv[1])
