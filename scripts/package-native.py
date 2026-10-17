#!/usr/bin/env python3
"""Build and stage an unsigned app copy. Never installs, activates, or signs an extension."""
import argparse
import os
import plistlib
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser()
p.add_argument('--app', type=Path, default=ROOT / 'src-tauri/target/debug/bundle/macos/FlowGuard.app')
p.add_argument('--output', type=Path, default=ROOT / 'native/build/FlowGuard.app')
p.add_argument('--team-id', default='UNSIGNED')
args = p.parse_args()
if args.team_id != 'UNSIGNED' and (len(args.team_id) != 10 or not args.team_id.isascii() or not args.team_id.isalnum() or args.team_id != args.team_id.upper()):
    p.error('team-id must be a 10-character Apple Team ID')
app = args.app.resolve()
output = args.output.resolve()
# Only replace the designated generated staging directory, never the input app.
if output != ROOT / 'native/build/FlowGuard.app' or output == app:
    p.error('output must be the generated native/build/FlowGuard.app directory')
if not (app / 'Contents/Info.plist').is_file():
    p.error('build the Tauri .app first')
scratch = ROOT / 'native/build/swift'
env = os.environ.copy()
env['CLANG_MODULE_CACHE_PATH'] = str(ROOT / 'native/build/clang-cache')
env['SWIFTPM_MODULECACHE_OVERRIDE'] = str(ROOT / 'native/build/module-cache')
command = ['swift', 'build', '--package-path', str(ROOT / 'native/FlowGuardFilter'), '--scratch-path', str(scratch), '--cache-path', str(ROOT / 'native/build/cache'), '--config-path', str(ROOT / 'native/build/config'), '--security-path', str(ROOT / 'native/build/security'), '--disable-sandbox', '-c', 'release', '-debug-info-format', 'none']
subprocess.run(command, check=True, env=env)
bin_path = Path(subprocess.check_output(command + ['--show-bin-path'], env=env, text=True).strip())
if output.exists():
    shutil.rmtree(output)
shutil.copytree(app, output)
# This staging copy is intentionally unsigned. Do not keep stale bundle signatures.
shutil.rmtree(output / 'Contents/_CodeSignature', ignore_errors=True)
extension = output / 'Contents/Library/SystemExtensions/com.flowguard.filter.systemextension'
(extension / 'Contents/MacOS').mkdir(parents=True)
shutil.copy2(bin_path / 'FlowGuardSystemExtension', extension / 'Contents/MacOS/FlowGuardSystemExtension')
shutil.copy2(bin_path / 'FlowGuardBridgeCLI', output / 'Contents/MacOS/FlowGuardBridge')
shutil.copy2(bin_path / 'FlowGuardFirewallHelper', output / 'Contents/MacOS/FlowGuardFirewallHelper')
daemons = output / 'Contents/Library/LaunchDaemons'
daemons.mkdir(parents=True, exist_ok=True)
with (daemons / 'com.flowguard.firewall-helper.plist').open('wb') as f:
    plistlib.dump({'Label': 'com.flowguard.firewall-helper',
                  'BundleProgram': 'Contents/MacOS/FlowGuardFirewallHelper',
                  'ProgramArguments': ['FlowGuardFirewallHelper', '--daemon'],
                  'MachServices': {'com.flowguard.firewall-helper': True},
                  'ProcessType': 'Interactive'}, f)
frameworks = output / 'Contents/Frameworks'
frameworks.mkdir(parents=True, exist_ok=True)
shutil.copy2(bin_path / 'libFlowGuardHostRuntime.dylib', frameworks / 'libFlowGuardHostRuntime.dylib')
with (ROOT / 'native/FlowGuardFilter/Packaging/Extension-Info.plist').open('rb') as f:
    info = plistlib.load(f)
with (output / 'Contents/Info.plist').open('rb') as f:
    version_info = plistlib.load(f)
info['CFBundleShortVersionString'] = version_info['CFBundleShortVersionString']
info['CFBundleVersion'] = version_info['CFBundleVersion']
info['NetworkExtension']['NEMachServiceName'] = args.team_id + '.com.flowguard.filter'
with (extension / 'Contents/Info.plist').open('wb') as f:
    plistlib.dump(info, f)
with (output / 'Contents/Info.plist').open('rb') as f:
    host_info = plistlib.load(f)
host_info['LSMinimumSystemVersion'] = '13.0'
host_info['NSSystemExtensionUsageDescription'] = 'FlowGuard uses a local network extension to apply your app connection rules.'
with (output / 'Contents/Info.plist').open('wb') as f:
    plistlib.dump(host_info, f)
local_network_descriptions = {
    'en': 'FlowGuard discovers devices on the LAN you select using neighbor information, IPv4 response checks, optional name lookup, manual TCP port checks and service response checks.',
    'ja': 'FlowGuardは、選択したLAN上の機器を近隣情報・IPv4応答確認・任意の名前取得と、手動のTCPポート・サービス応答確認で調査します。',
    'zh-Hans': 'FlowGuard通过邻居信息、IPv4响应检查、可选名称查询、手动TCP端口与服务响应检查调查您所选LAN中的设备。',
}
descriptions = {
    'en': 'FlowGuard uses a local network extension to apply your app connection rules.',
    'ja': 'FlowGuardは、このMac上のネットワーク拡張機能でアプリごとの通信ルールを適用します。',
    'zh-Hans': 'FlowGuard使用此Mac上的本地网络扩展来应用各个应用的连接规则。',
}
for locale, description in descriptions.items():
    resources = output / 'Contents/Resources' / (locale + '.lproj')
    resources.mkdir(parents=True, exist_ok=True)
    (resources / 'InfoPlist.strings').write_text('"NSSystemExtensionUsageDescription" = "' + description + '";\n' + '"NSLocalNetworkUsageDescription" = "' + local_network_descriptions[locale] + '";\n', encoding='utf-8')
subprocess.run([str(output / 'Contents/MacOS/FlowGuardBridge'), '--preflight', str(extension)], check=True)
print(f'Unsigned staging bundle: {output}')
