// パス: src/core/investigation-graph.ts
import type {Snapshot, DeviceChange} from './types';
import {samePortTarget} from './port-sheet';
export type InvestigationNode = 'device' | 'ports' | 'services' | 'diagnostics';
export type Segment = {key:string; count:number; tone:'observed'|'unknown'|'missing'|'warning'};
const count = (rows:{status:string}[], states:string[]) => rows.filter(r=>states.includes(r.status)).length;
export function investigationGraph(snapshot:Snapshot,row:DeviceChange) {
  const {inventory,discovery,network,ports,services,diagnostics}=snapshot;
  const device=row.device;
  const anchorFamily=inventory.localAddress.includes(':')?'IPv6':'IPv4';
  const anchor=network.error===null&&network.addresses.some(a=>a.up&&a.netmask===inventory.netmask&&samePortTarget(a,{target:inventory.localAddress,interface:device.interface,family:anchorFamily}));
  const current=anchor&&inventory.generation>0&&row.kind!=='not_seen'&&['completed','partial'].includes(discovery.stage)&&!discovery.errors.includes('network_changed')&&discovery.devices.some(d=>d.id===device.id&&samePortTarget(d,{target:device.address,interface:device.interface,family:device.family}));
  const portSource=network.addresses.some(a=>a.up&&a.netmask===ports.sourceNetmask&&samePortTarget(a,{...ports,target:ports.sourceAddress}));
  const hasPorts=current&&ports.operation>0&&ports.generation===inventory.generation&&samePortTarget(device,ports)&&portSource&&ports.error!=='network_changed';
  const hasServices=hasPorts&&ports.stage==='completed'&&!ports.error&&services.operation>0&&services.portOperation===ports.operation&&services.generation===inventory.generation&&samePortTarget(device,services)&&services.sourceNetmask===ports.sourceNetmask&&samePortTarget({address:services.sourceAddress,interface:services.interface,family:services.family},{...ports,target:ports.sourceAddress})&&services.error!=='network_changed';
  const reports=hasServices&&services.stage==='completed'&&!services.error?(diagnostics?.reports??[]).filter(r=>r.generation===inventory.generation&&r.serviceOperation===services.operation&&samePortTarget(device,{...r,family:services.family})):[];
  const report=reports.find(r=>r.id===diagnostics?.selectedReport)??[...reports].sort((a,b)=>b.id-a.id)[0];
  // Rust omits findings for unselected reports; an empty payload must not become a zero finding result.
  const reportLoaded=!!report&&diagnostics?.selectedReport===report.id;
  const open=hasPorts?count(ports.rows,['open']):0;
  const refused=hasPorts?count(ports.rows,['closed']):0;
  const serviceConfirmed=hasServices?services.rows.filter(r=>r.status==='confirmed'&&r.service!==null&&(r.method!=='https'||r.tlsDetails?.verification==='verified')).length:0;
  const unsupported=hasServices?count(services.rows,['unsupported']):0;
  const portSegments:Segment[]=hasPorts?[
    {key:'open',count:open,tone:'observed'},
    {key:'refused',count:refused,tone:'missing'},
    {key:'unknown',count:ports.rows.length-open-refused,tone:'unknown'},
    {key:'remaining',count:Math.max(0,ports.total-ports.rows.length),tone:'missing'}]:[];
  const serviceSegments:Segment[]=hasServices?[
    {key:'confirmed',count:serviceConfirmed,tone:'observed'},
    {key:'unknown',count:services.rows.length-serviceConfirmed-unsupported,tone:'unknown'},
    {key:'unsupported',count:unsupported,tone:'missing'},
    {key:'remaining',count:Math.max(0,services.total-services.rows.length),tone:'missing'}]:[];
  const findings=reportLoaded?report!.findings:[];
  const diagnosticSegments:Segment[]=reportLoaded?[
    {key:'observed',count:findings.filter(f=>f.certainty==='observed').length,tone:'observed'},
    {key:'unverified',count:findings.filter(f=>!['observed','not_performed'].includes(f.certainty)).length,tone:'unknown'},
    {key:'not_performed',count:findings.filter(f=>f.certainty==='not_performed').length,tone:'missing'}]:[];
  return {current,hasPorts,hasServices,report,reportLoaded,reports:reports.length,open,serviceConfirmed,portSegments,serviceSegments,diagnosticSegments,
    warning:findings.filter(f=>f.severity==='warning').length,
    info:findings.filter(f=>f.severity==='info').length};
}
export const investigationWords={
 ja:{title:'調査のつながり',device:'機器',ports:'TCPポート',services:'サービス',diagnostics:'診断',open:'開放',refused:'接続拒否',unknown:'不確定',remaining:'未完了',confirmed:'応答確認',unsupported:'未対応・未実施',observed:'観測済み',unverified:'要確認',not_performed:'未実施',warning:'注意',info:'参考',none:'関連記録なし',stale:'旧収集・関連無効',record:'記録あり',load:'内訳を読み込む',loading:'読込中',failed:'読込失敗',view:'結果を開く',select:'ノードを選択して内訳を見る',reports:'レポート',items:'項目',scope:'収集世代',operation:'操作',time:'観測時刻',completed:'完了記録',running:'確認中',cancelling:'停止中',cancelled:'停止・未完了',interrupted:'中断・失敗',empty:'内訳はまだありません',help:'同じ収集世代・対象IP・インターフェース・送信元とTCP／サービス操作が一致する記録だけを関連付けます。線は観測根拠の関係であり、実際のパケットや防御の動作ではありません。未確認・未実施は安全を意味しません。グラフ表示で追加の通信や診断は開始しません。',deviceHelp:'1つのアドレス記録です。名前やMACが同じでも別の機器・アドレスを統合しません。',portsHelp:'開放・拒否はその時点の経路からの応答。不確定と未完了を閉鎖と扱いません。',servicesHelp:'サービスの応答記録です。製品情報は自己申告で、未取得を正常とは判定しません。',diagnosticsHelp:'診断の確かさと重要度は別です。未実施を含めて表示し、安全という総合判定はしません。'},
 en:{title:'Investigation trail',device:'Device',ports:'TCP ports',services:'Services',diagnostics:'Findings',open:'Open',refused:'Refused',unknown:'Uncertain',remaining:'Incomplete',confirmed:'Response verified',unsupported:'Unsupported / unperformed',observed:'Observed',unverified:'Needs review',not_performed:'Not performed',warning:'Warning',info:'Informational',none:'No related records',stale:'Old collection · unlinked',record:'Records available',load:'Load breakdown',loading:'Loading',failed:'Load failed',view:'Open results',select:'Select a node to explore',reports:'reports',items:'items',scope:'Collection',operation:'Operation',time:'Observed at',completed:'Completed record',running:'Checking',cancelling:'Stopping',cancelled:'Stopped · incomplete',interrupted:'Interrupted / failed',empty:'No breakdown available',help:'Link only matching collection, target IP, interface, source and TCP/service operations. Lines represent evidence relationships, not packets or enforcement. Uncertain and unperformed never mean safe. Displaying this graph starts no traffic or assessment.',deviceHelp:'One address record. Matching names or MACs do not merge separate devices or addresses.',portsHelp:'Open or refused reflects a response from that route and time. Uncertain and incomplete are not closed.',servicesHelp:'Collected service responses. Product details are self-reported; missing evidence never means healthy.',diagnosticsHelp:'Certainty and severity are separate. Unperformed checks remain visible; there is no overall safe verdict.'},
 'zh-CN':{title:'调查关联',device:'设备',ports:'TCP端口',services:'服务',diagnostics:'诊断',open:'开放',refused:'连接拒绝',unknown:'不确定',remaining:'未完成',confirmed:'响应确认',unsupported:'不支持／未执行',observed:'已观测',unverified:'需确认',not_performed:'未执行',warning:'注意',info:'参考',none:'无关联记录',stale:'旧采集・关联失效',record:'有记录',load:'加载明细',loading:'加载中',failed:'加载失败',view:'打开结果',select:'选择节点查看明细',reports:'报告',items:'项目',scope:'采集代次',operation:'操作',time:'观测时间',completed:'完成记录',running:'检查中',cancelling:'停止中',cancelled:'已停止・未完成',interrupted:'中断／失败',empty:'暂无明细',help:'仅关联采集代次、目标IP、接口、源地址和TCP／服务操作一致的记录。连线表示观测证据的关系，不代表数据包或防护动作。未确认或未执行不代表安全。显示此图不会启动通信或诊断。',deviceHelp:'一个地址记录。即使名称或MAC相同，也不合并不同设备或地址。',portsHelp:'开放或拒绝仅表示当时路径的响应。不确定和未完成不等于关闭。',servicesHelp:'已采集的服务响应。产品信息由设备自行报告，缺少记录不等于正常。',diagnosticsHelp:'证据确定性与重要度分开显示，保留未执行项目，不给出整体安全结论。'}
};
