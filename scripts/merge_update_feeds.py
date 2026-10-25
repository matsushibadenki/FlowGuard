#!/usr/bin/env python3
"""Merge both verified architecture stages. Default is local; publication is explicit."""
import argparse, base64, hashlib, json, re, subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
REPO='matsushibadenki/FlowGuard'

def merge_manifests(feeds):
    if len(feeds)!=2: raise ValueError('Both architecture manifests are required')
    version=feeds[0].get('version');notes=feeds[0].get('notes','')
    if not isinstance(version,str) or not re.fullmatch(r'\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?',version) or not isinstance(notes,str) or len(notes)>8192: raise ValueError('Invalid version or notes')
    platforms={}
    for feed in feeds:
        if feed.get('version')!=version or feed.get('notes','')!=notes or not isinstance(feed.get('platforms'),dict) or len(feed['platforms'])!=1: raise ValueError('Inconsistent feed metadata')
        for key,asset in feed['platforms'].items():
            if key not in ['darwin-aarch64','darwin-x86_64'] or key in platforms or not isinstance(asset,dict): raise ValueError('Unexpected or duplicate target')
            arch=key.removeprefix('darwin-');name=f'FlowGuard_{version}_{arch}.app.tar.gz'
            if asset.get('url')!=f'https://github.com/{REPO}/releases/download/v{version}/{name}': raise ValueError('Unexpected asset URL')
            sig=asset.get('signature','')
            if not isinstance(sig,str) or not 1<len(sig)<=8192: raise ValueError('Invalid signature')
            try: decoded=base64.b64decode(sig,validate=True).decode('utf-8')
            except (ValueError,UnicodeError) as e: raise ValueError('Invalid signature encoding') from e
            if 'trusted comment:' not in decoded or f'version:{version}' not in decoded: raise ValueError('Missing signed version')
            platforms[key]=asset
    if set(platforms)!= {'darwin-aarch64','darwin-x86_64'}: raise ValueError('Missing architecture')
    return {'version':version,'notes':notes,'pub_date':feeds[0].get('pub_date'),'platforms':platforms}

def validate_stage(directory,feed):
    target=next(iter(feed['platforms']));arch=target.removeprefix('darwin-');version=feed['version'];name=f'FlowGuard_{version}_{arch}.app.tar.gz'
    report=json.loads((directory/'notarization.json').read_text())
    if report.get('status')!='Accepted': raise ValueError('Notarization must be Accepted')
    sums={line.split('  ',1)[1]:line.split('  ',1)[0] for line in (directory/'SHA256SUMS').read_text().splitlines() if '  ' in line}
    archive=directory/name
    if archive.is_symlink() or not archive.is_file() or archive.stat().st_size>200*1024*1024 or sums.get(name)!=hashlib.sha256(archive.read_bytes()).hexdigest(): raise ValueError('Archive checksum mismatch')
    if (directory/(name+'.sig')).read_text().strip()!=feed['platforms'][target]['signature']: raise ValueError('Detached signature mismatch')
    return archive

def run(args,capture=False):
    p=subprocess.run([str(a) for a in args],cwd=ROOT,check=True,capture_output=capture,text=True)
    return p.stdout if capture else None

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--stage',action='append',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--channel',choices=['stable','beta'],default='stable');p.add_argument('--publish-draft',action='store_true');p.add_argument('--promote-beta',action='store_true');args=p.parse_args()
    directories=[d.resolve() for d in args.stage]
    feeds=[json.loads((d/'latest.json').read_text()) for d in directories];merged=merge_manifests(feeds)
    if args.channel=='stable' and '-' in merged['version']: p.error('Prereleases cannot enter stable')
    assets=[validate_stage(d,f) for d,f in zip(directories,feeds)]
    if args.output.exists(): p.error('Output must be new; existing feeds are not overwritten')
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(merged,ensure_ascii=False,indent=2)+'\n')
    if args.publish_draft or args.promote_beta:
        for archive in assets:
            run(['cargo','run','--manifest-path','src-tauri/Cargo.toml','--example','verify_update_artifact','--no-default-features','--offline','--',archive,merged['version'],args.channel])
        for directory in directories:
            app=directory/'FlowGuard.app';run(['codesign','--verify','--deep','--strict',app]);run(['xcrun','stapler','validate',app]);run(['spctl','--assess','--type','execute',app])
    tag='v'+merged['version']
    if args.publish_draft:
        if subprocess.run(['gh','release','view',tag,'--repo',REPO],capture_output=True).returncode==0: p.error('Release exists; immutable assets are not overwritten')
        if args.output.name!='latest.json': p.error('Publishable feed must be named latest.json')
        run(['gh','release','create',tag,'--repo',REPO,'--verify-tag','--draft','--title','FlowGuard '+merged['version'],*(['--prerelease'] if args.channel=='beta' else []),*assets,*[str(a)+'.sig' for a in assets],args.output])
    if args.promote_beta:
        if args.channel!='beta' or args.output.name!='latest.json': p.error('Beta promotion needs beta/latest.json')
        state=json.loads(run(['gh','release','view',tag,'--repo',REPO,'--json','isDraft'],capture=True))
        if state.get('isDraft') is not False: p.error('Publish the immutable version release before promoting its feed')
        run(['gh','release','view','beta','--repo',REPO])
        # Only the explicit beta channel pointer may be replaced; archives stay versioned.
        run(['gh','release','upload','beta',args.output,'--repo',REPO,'--clobber'])
    print('Merged targets:',', '.join(sorted(merged['platforms'])))
if __name__=='__main__':main()
