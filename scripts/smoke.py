#!/usr/bin/env python3
"""Compile real ELF fixtures and exercise the CLI and independent readelf oracle.
Run from any directory after cargo build: python3 scripts/smoke.py
External tools are used only for verification, never by Vecta itself.
"""
import base64
import json
from pathlib import Path
import re
import subprocess
import tempfile
import zlib

ROOT = Path(__file__).resolve().parents[1]
VECTA = ROOT / 'target/debug/vecta'

def run(*args, code=0):
    result = subprocess.run([str(a) for a in args], capture_output=True, text=True)
    assert result.returncode == code, (args, result.returncode, result.stdout, result.stderr)
    return result.stdout

with tempfile.TemporaryDirectory(prefix='vecta-smoke-') as temp:
    temp = Path(temp)
    for name, size in [('base', 32), ('head', 131072)]:
        source = temp / f'{name}.c'
        source.write_text('#include <stdio.h>\n'
            f'__attribute__((used)) const unsigned char payload[{size}] = {{[0]=17,[{size-1}]=23}};\n'
            'int main(void) { puts("Vecta fixture"); return payload[0] == 0; }\n')
        command = ['cc', '-O0', '-g', str(source), '-o', str(temp/name)]
        if name == 'head':
            command += ['-Wl,--no-as-needed', '-lm']
        run(*command)
    base = json.loads(run(VECTA, 'inspect', temp/'base'))
    head = json.loads(run(VECTA, 'inspect', temp/'head'))
    assert base == json.loads(run(VECTA, 'inspect', temp/'base'))
    assert base['file_size'] == (temp/'base').stat().st_size
    for name, snapshot in [('base',base),('head',head)]:
        # Check every logical section byte count against GNU readelf.
        sections = {s['name']:s['size'] for s in snapshot['sections']}
        for line in run('readelf', '-SW', temp/name).splitlines():
            match = re.match(r'\s*\[\s*\d+\]\s+(\S+)\s+\S+\s+[0-9a-fA-F]+\s+[0-9a-fA-F]+\s+([0-9a-fA-F]+)', line)
            if match:
                assert sections[match[1]] == int(match[2],16), line
        versions = re.findall(r'Name:\s+GLIBC_([0-9.]+)',run('readelf','--version-info',temp/name))
        assert snapshot['abi']['GLIBC'] == max(versions,key=lambda v:tuple(map(int,v.split('.'))))
        (temp/f'{name}.json').write_text(json.dumps(snapshot))
    report = json.loads(run(VECTA,'diff',temp/'base',temp/'head','--format','json','--viewer-url','https://example.com/view'))
    saved = json.loads(run(VECTA,'diff',temp/'base.json',temp/'head.json','--format','json'))
    assert report['diff'] == saved['diff']
    assert 'libm.so.6' in report['diff']['libraries_added']
    assert report['diff']['file']['delta'] == head['file_size']-base['file_size']
    assert any(s['raw_name']=='payload' and s['delta']==131072-32 for s in report['diff']['symbols'])
    url = report['share_url']
    encoded = url.split('#z=')[1]
    payload = json.loads(zlib.decompress(base64.urlsafe_b64decode(encoded+'='*(-len(encoded)%4)),wbits=-15))
    assert payload['diff']['file'] == report['diff']['file'] and len(url)<=8000
    run(VECTA,'gate',temp/'base',temp/'base','--max-growth','0')
    run(VECTA,'gate',temp/'base',temp/'head','--max-growth','0',code=1)
    run(VECTA,'gate',temp/'base',temp/'head','--forbid-new-libraries',code=1)
    run(VECTA,'gate',temp/'base',temp/'head','--section-budget','.rodata=0',code=1)
    run(VECTA,'gate',temp/'base',temp/'head','--max-glibc','2.0',code=1)
    run(VECTA,'gate',temp/'base',temp/'head',code=2)
    run(VECTA,'inspect',temp/'missing',code=2)
    run(VECTA,'diff',temp/'base',temp/'head','--format','markdown','--output',temp/'report.md')
    assert '| Section |' in (temp/'report.md').read_text()
    run('strip','--strip-all','-o',temp/'stripped',temp/'base')
    stripped = json.loads(run(VECTA,'inspect',temp/'stripped'))
    assert any('dynamic symbols' in w for w in stripped['warnings'])
    assert stripped['abi'] == base['abi']
    for fixture in ['macho64.bin','pe64.bin']:
        run(VECTA,'inspect',ROOT/'crates/vecta-core/tests/fixtures'/fixture)
print('PASS: deterministic inspect, exact section/symbol sizes, ABI oracle, binary/snapshot parity, gates, Markdown, stripped fallback, cross-format fixtures, share decode')
