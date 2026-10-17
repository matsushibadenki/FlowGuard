#!/usr/bin/env python3
"""Build, Developer-ID sign, notarize and prepare updater assets. Never uploads secrets.
Default preparation is local only. Apple submission requires --submit-prepared and an exact archive hash.
Publishing separately requires --publish.
"""
import argparse, base64, datetime, hashlib, json, os, platform, plistlib, re, shutil, subprocess, tarfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
REPO = 'matsushibadenki/FlowGuard'

def run(args, capture=False):
    result = subprocess.run([str(x) for x in args], cwd=ROOT, capture_output=capture, text=capture)
    if result.returncode:
        raise RuntimeError('Release step failed: ' + str(args[0]) + ' (credential output withheld)' if capture else 'Release step failed')
    return result.stdout if capture else None

def manifest(version, architecture, signature, notes):
    if not re.fullmatch(r'\d+\.\d+\.\d+', version) or architecture not in ['aarch64', 'x86_64']:
        raise ValueError('Invalid stable version or architecture')
    name = f'FlowGuard_{version}_{architecture}.app.tar.gz'
    return {'version': version, 'notes': notes[:8192], 'pub_date': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'platforms': {f'darwin-{architecture}': {'signature': signature.strip(), 'url': f'https://github.com/{REPO}/releases/download/v{version}/{name}'}}}

def bundle_digest(app):
    digest=hashlib.sha256()
    for path in sorted(app.rglob('*')):
        if path.is_symlink(): raise RuntimeError('Symlinks are unsupported by this release policy')
        digest.update((b'F' if path.is_file() else b'D')+b'\0')
        digest.update(str(path.relative_to(app)).encode()+b'\0')
        digest.update(str(path.stat().st_mode & 0o777).encode()+b'\0')
        if path.is_file(): digest.update(hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()

def validate_prepared_payload(record, expected, app, archive, version, architecture, team):
    if not isinstance(record,dict): raise ValueError('Invalid preparation record')
    if not isinstance(expected,str) or not re.fullmatch(r'[a-f0-9]{64}',expected): raise ValueError('An approved archive SHA-256 is required')
    if record.get('version')!=version or record.get('team_id')!=team or record.get('architecture')!=architecture or record.get('sha256')!=expected or hashlib.sha256(archive.read_bytes()).hexdigest()!=expected or record.get('bundle_sha256')!=bundle_digest(app):
        raise ValueError('Prepared payload differs from the explicitly approved archive')

def write_verification_assets(directory):
    # Local only: these are deliberately outside the publishable release asset directory.
    directory=directory.resolve()
    if ROOT/'release' not in directory.parents: raise ValueError('Use a local generated release directory')
    record=json.loads((directory/'preparation.json').read_text())
    app=directory/'FlowGuard.app'
    validate_prepared_payload(record,record['sha256'],app,directory/'notarization.zip',record['version'],record['architecture'],record['team_id'])
    output=directory/'verification'
    output.mkdir(exist_ok=False)
    version=record['version'];architecture=record['architecture']
    archive=output/f'FlowGuard_{version}_{architecture}.app.tar.gz'
    with tarfile.open(archive,'w:gz',format=tarfile.USTAR_FORMAT) as tar: tar.add(app,arcname='FlowGuard.app')
    run(['./node_modules/.bin/tauri','signer','sign','--app-version',version,'--private-key-path',ROOT/'.release-secrets/updater.key',archive],capture=True)
    signature=Path(str(archive)+'.sig').read_text()
    (output/'latest.json').write_text(json.dumps(manifest(version,architecture,signature,'Local verification only; not a public feed'),indent=2)+'\n')
    print('Local verification assets (not notarized or publishable):',output)

def main():
    p = argparse.ArgumentParser()
    p.add_argument('--identity', required=True, help='Developer ID Application certificate SHA-1 fingerprint')
    p.add_argument('--team-id', required=True)
    p.add_argument('--notary-profile', default='flowguard')
    p.add_argument('--host-profile', type=Path)
    p.add_argument('--extension-profile', type=Path)
    p.add_argument('--observe-only', action='store_true', help='Distribute host without the unprovisioned System Extension')
    p.add_argument('--notes', type=Path, required=True)
    p.add_argument('--resume-prepared-app', action='store_true', help='Resume signing a staged app after an interrupted preparation')
    p.add_argument('--submit-prepared', action='store_true', help='Submit the already prepared immutable archive to Apple, then finish updater assets')
    p.add_argument('--expected-notarization-sha256', help='Explicitly approved SHA-256 of the prepared Apple submission archive')
    p.add_argument('--publish', action='store_true', help='Publish already verified assets to GitHub Releases')
    args = p.parse_args()
    if platform.system() != 'Darwin': p.error('macOS is required')
    if not re.fullmatch(r'[A-Fa-f0-9]{40}', args.identity) or not re.fullmatch(r'[A-Z0-9]{10}', args.team_id): p.error('Invalid identity/team')
    if not args.observe_only and (not args.host_profile or not args.extension_profile): p.error('Developer ID host and extension provisioning profiles are required. --observe-only explicitly prepares a host-only release.')
    config = json.loads((ROOT/'src-tauri/tauri.conf.json').read_text())
    version = config['version']
    if not re.fullmatch(r'\d+\.\d+\.\d+', version): p.error('A stable release version is required')
    build_version = config['bundle']['macOS'].get('bundleVersion', '')
    if not re.fullmatch(r'[1-9][0-9]*', build_version): p.error('An explicit positive integer macOS bundleVersion is required')
    if args.publish and not args.submit_prepared: p.error('Publishing requires an explicitly submitted and verified prepared release')
    if args.submit_prepared and (args.resume_prepared_app or not args.expected_notarization_sha256): p.error('Submission requires the approved archive SHA-256 and cannot rebuild or re-sign')
    if json.loads((ROOT/'package.json').read_text())['version'] != version: p.error('Version mismatch')
    import tomllib
    if tomllib.loads((ROOT/'src-tauri/Cargo.toml').read_text())['package']['version'] != version: p.error('Version mismatch')
    key = ROOT/'.release-secrets/updater.key'
    if not key.is_file() or not Path(str(key)+'.pub').is_file(): p.error('Local updater signing key is missing')
    if Path(str(key)+'.pub').read_text().strip() != config['plugins']['updater']['pubkey']: p.error('Updater key does not match the app public key')
    if not args.observe_only:
        for profile, identifier in [(args.host_profile,'com.flowguard.desktop'),(args.extension_profile,'com.flowguard.filter')]:
            value=plistlib.loads(run(['security','cms','-D','-i',profile],capture=True).encode())
            entitlements=value.get('Entitlements',{})
            app_id=entitlements.get('com.apple.application-identifier',entitlements.get('application-identifier'))
            expires=value.get('ExpirationDate')
            if args.team_id not in value.get('TeamIdentifier',[]) or app_id!=args.team_id+'.'+identifier or not value.get('ProvisionsAllDevices') or entitlements.get('get-task-allow') or not expires or expires.replace(tzinfo=datetime.timezone.utc)<=datetime.datetime.now(datetime.timezone.utc):
                p.error('Invalid/expired Developer ID provisioning profile for '+identifier)
            if 'content-filter-provider-systemextension' not in entitlements.get('com.apple.developer.networking.networkextension',[]):
                p.error('Network Extension capability missing in '+identifier)
            if identifier=='com.flowguard.desktop' and not entitlements.get('com.apple.developer.system-extension.install'):
                p.error('System Extension install capability missing in host profile')
            if args.identity.upper() not in [hashlib.sha1(cert).hexdigest().upper() for cert in value.get('DeveloperCertificates',[])]:
                p.error('Signing certificate is not included in profile')
    notes = args.notes.read_text()
    architecture = 'aarch64' if platform.machine()=='arm64' else 'x86_64'
    output = ROOT/'release'/version/architecture
    app = output/'FlowGuard.app'
    notary_zip = output/'notarization.zip'
    preparation = output/'preparation.json'
    if not args.submit_prepared:
        if args.resume_prepared_app:
            if not (output/'FlowGuard.app/Contents/Info.plist').is_file() or (output/'latest.json').exists(): p.error('No unfinished app preparation to resume')
        else:
            if output.exists(): p.error('Release output already exists; preserve it or resume unfinished preparation explicitly')
            output.mkdir(parents=True)
            # App is bundled first; every nested binary must be embedded before signing.
            run(['npm','run','tauri','--','build','--bundles','app','--no-sign','--ci'])
            host = ROOT/'src-tauri/target/release/bundle/macos/FlowGuard.app'
            if args.observe_only:
                shutil.copytree(host, output/'FlowGuard.app')
            else:
                run(['python3','scripts/package-native.py','--app',host,'--team-id',args.team_id])
                shutil.copytree(ROOT/'native/build/FlowGuard.app', output/'FlowGuard.app')
        app = output/'FlowGuard.app'
        if not args.observe_only:
            for label, identifier in [('Host','com.flowguard.desktop'),('Extension','com.flowguard.filter')]:
                ent = plistlib.loads((ROOT/f'native/FlowGuardFilter/Packaging/{label}.entitlements').read_bytes())
                ent['com.apple.application-identifier'] = args.team_id+'.'+identifier
                ent['com.apple.developer.team-identifier'] = args.team_id
                (output/f'{label}.entitlements').write_bytes(plistlib.dumps(ent))
            extension = app/'Contents/Library/SystemExtensions/com.flowguard.filter.systemextension'
            shutil.copy2(args.host_profile, app/'Contents/embedded.provisionprofile')
            shutil.copy2(args.extension_profile, extension/'Contents/embedded.provisionprofile')
            run(['codesign','--force','--options','runtime','--timestamp','--sign',args.identity,'--identifier','com.flowguard.bridge',app/'Contents/MacOS/FlowGuardBridge'])
            run(['codesign','--force','--options','runtime','--timestamp','--sign',args.identity,app/'Contents/Frameworks/libFlowGuardHostRuntime.dylib'])
            run(['codesign','--force','--options','runtime','--timestamp','--sign',args.identity,'--identifier','com.flowguard.firewall-helper',app/'Contents/MacOS/FlowGuardFirewallHelper'])
            run(['codesign','--force','--options','runtime','--timestamp','--sign',args.identity,'--entitlements',output/'Extension.entitlements',extension])
        command = ['codesign','--force','--options','runtime','--timestamp','--sign',args.identity]
        if not args.observe_only: command += ['--entitlements',output/'Host.entitlements']
        run(command+[app])
        run(['codesign','--verify','--deep','--strict',app])
        with (app/'Contents/Info.plist').open('rb') as f: info=plistlib.load(f)
        if info['CFBundleShortVersionString']!=version or info['CFBundleIdentifier']!='com.flowguard.desktop' or info['CFBundleVersion']!=build_version: raise RuntimeError('Bundle identity/version mismatch')
        identities = [(app, 'com.flowguard.desktop')]
        if not args.observe_only:
            with (extension/'Contents/Info.plist').open('rb') as f: extension_info=plistlib.load(f)
            if extension_info['CFBundleShortVersionString']!=version or extension_info['CFBundleVersion']!=build_version: raise RuntimeError('Extension bundle version mismatch')
            identities += [(extension, 'com.flowguard.filter'), (app/'Contents/MacOS/FlowGuardBridge', 'com.flowguard.bridge'), (app/'Contents/MacOS/FlowGuardFirewallHelper', 'com.flowguard.firewall-helper')]
        for binary, identifier in identities:
            requirement=f'anchor apple generic and identifier "{identifier}" and certificate leaf[subject.OU] = "{args.team_id}"'
            run(['codesign','--verify','--strict','-R','='+requirement,binary])
        if not args.observe_only:
            preflight=json.loads(run([app/'Contents/MacOS/FlowGuardBridge','--preflight',extension],capture=True))
            if preflight.get('signed') is not True or preflight.get('packaged') is not True: raise RuntimeError('Signed bridge preflight failed')
        notary_zip = output/'notarization.zip'
        run(['ditto','-c','-k','--keepParent',app,notary_zip])
        record={'version':version,'architecture':architecture,'team_id':args.team_id,'sha256':hashlib.sha256(notary_zip.read_bytes()).hexdigest(),'bytes':notary_zip.stat().st_size,'bundle_sha256':bundle_digest(app),'destination':'Apple Developer notarization service (notarytool)'}
        preparation.write_text(json.dumps(record,indent=2)+'\n')
        print('Prepared locally; no Apple submission or publication:',output)
        print('Apple submission archive SHA-256:',record['sha256'])
        return
    if not preparation.is_file() or not notary_zip.is_file() or (output/'latest.json').exists(): p.error('An unfinished local preparation is required')
    record=json.loads(preparation.read_text())
    try: validate_prepared_payload(record,args.expected_notarization_sha256,app,notary_zip,version,architecture,args.team_id)
    except ValueError as error: p.error(str(error))
    run(['codesign','--verify','--deep','--strict',app])
    # Credentials are read from Keychain; no password is put in argv or release metadata.
    report=json.loads(run(['xcrun','notarytool','submit',notary_zip,'--keychain-profile',args.notary_profile,'--wait','--output-format','json'],capture=True))
    if report.get('status')!='Accepted': raise RuntimeError('Notarization was not accepted; no release is published')
    (output/'notarization.json').write_text(json.dumps({'id':report['id'],'status':report['status']},indent=2)+'\n')
    run(['xcrun','stapler','staple',app]);run(['xcrun','stapler','validate',app]);run(['spctl','--assess','--type','execute',app])
    name=f'FlowGuard_{version}_{architecture}.app.tar.gz';archive=output/name
    # The installer expects one .app root. Current bundles have no framework symlinks.
    if any(p.is_symlink() for p in app.rglob('*')): raise RuntimeError('Symlinks are unsupported by the current update archive policy')
    with tarfile.open(archive,'w:gz',format=tarfile.USTAR_FORMAT) as tar: tar.add(app,arcname='FlowGuard.app')
    run(['./node_modules/.bin/tauri','signer','sign','--app-version',version,'--private-key-path',key,archive],capture=True)
    signature=Path(str(archive)+'.sig').read_text()
    # A downloadable zip includes the stapled app for first installation.
    zip_name=output/f'FlowGuard_{version}_{architecture}.zip'
    run(['ditto','-c','-k','--keepParent',app,zip_name])
    (output/'latest.json').write_text(json.dumps(manifest(version,architecture,signature,notes),ensure_ascii=False,indent=2)+'\n')
    (output/'SHA256SUMS').write_text('\n'.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}' for p in [archive,zip_name])+ '\n')
    if args.publish:
        # Never overwrite an existing version or release asset.
        existing=subprocess.run(['gh','release','view',f'v{version}','--repo',REPO],cwd=ROOT,capture_output=True)
        if existing.returncode==0: raise RuntimeError('Release already exists; assets will not be overwritten')
        run(['gh','release','create',f'v{version}','--repo',REPO,'--verify-tag','--title',f'FlowGuard {version}','--notes-file',args.notes,'--draft',archive,Path(str(archive)+'.sig'),zip_name,output/'latest.json',output/'SHA256SUMS'])
        run(['gh','release','edit',f'v{version}','--repo',REPO,'--draft=false'])
    print('Prepared verified release:',output)
if __name__=='__main__': main()
