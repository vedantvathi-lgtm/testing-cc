"""Minimal preprocessor for vello (#import name / #ifdef) and rerun (#import <path>) WGSL.
Imports inline once per file (include-guard semantics). Defines given via -D; undefined otherwise."""
import re, sys, pathlib
def prep(path, roots, defines, seen, out):
    stack=[True]
    for line in pathlib.Path(path).read_text().splitlines():
        s=line.strip()
        m=re.match(r'#(ifdef|ifndef)\s+(\w+)', s)
        if m:
            v=(m.group(2) in defines)==(m.group(1)=='ifdef'); stack.append(stack[-1] and v); continue
        if s.startswith('#else'): top=stack.pop(); stack.append(stack[-1] and not top if len(stack)>0 else not top); continue
        if s.startswith('#endif'): stack.pop(); continue
        if not stack[-1]: continue
        m=re.match(r'#import\s+<?([^>\s]+)>?', s)
        if m:
            name=m.group(1); cands=[pathlib.Path(path).parent/name]+[pathlib.Path(r)/name for r in roots]+[pathlib.Path(r)/(name+'.wgsl') for r in roots]
            f=next((c.resolve() for c in cands if c.is_file()), None)
            if f is None: out.append(f'// UNRESOLVED IMPORT {name}'); continue
            if f in seen: continue
            seen.add(f); prep(f, roots, defines, seen, out); continue
        out.append(line)
src, dst = sys.argv[1], sys.argv[2]; roots=[a[2:] for a in sys.argv[3:] if a.startswith('-I')]; defines={a[2:] for a in sys.argv[3:] if a.startswith('-D')}
out=[]; prep(src, roots, defines, {pathlib.Path(src).resolve()}, out); pathlib.Path(dst).write_text('\n'.join(out)+'\n')
