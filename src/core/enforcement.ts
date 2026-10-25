import {systemProtected} from './systemProtection';
// パス: src/core/enforcement.ts
import {validIdentity,identityKey} from './identity';
import type {ExtensionStatus,Rule} from './types';
import type {TextKey} from './i18n';
export const defaultExtension:ExtensionStatus={stage:'not_packaged',packaged:false,runtimeAvailable:false,signed:false,busy:false,instanceId:null,providerRevision:null,appliedRuleIds:[],error:null};
export function extensionLabel(stage:string):TextKey{
 const labels:Record<string,TextKey>={initial_sync:'initialPolicySync',disabling:'extensionDisabling',disabled:'extensionDisabled',removing:'extensionRemoving',removed:'extensionRemoved',invalid_initial_policy:'initialPolicyInvalid',not_packaged:'extensionNotPackaged',runtime_unavailable:'runtimeUnavailable',signature_required:'signatureRequired',inactive:'extensionInactive',activating:'extensionActivating',approval_required:'approvalRequired',restart_required:'restartRequired',configuration_saved:'configurationSaved',ready:'providerReady',applying:'rulesApplying',applied:'rulesApplied',stale:'policyStale',disconnected:'providerDisconnected',failed:'extensionFailed',configuration_conflict:'configurationConflict',busy:'extensionBusy'};
 return labels[stage]??'unknown';
}
export function ruleApplied(rule:Rule,status:ExtensionStatus){return !((rule.action==='block'||rule.action==='ask')&&systemProtected(rule))&&rule.source==='live'&&validIdentity(rule.identity)&&rule.appKey===identityKey(rule.identity)&&status.stage==='applied'&&!status.busy&&status.signed&&status.error===null&&!!status.instanceId&&status.providerRevision!==null&&status.appliedRuleIds.includes(rule.id);}
export function policyNotice(status:ExtensionStatus,mode:'live'|'demo'):TextKey{
 if(mode==='demo')return 'demoNeverApplied';
 if(status.stage==='applied')return 'acknowledgedPolicy';
 if(status.stage==='stale')return 'stalePolicyHelp';
 if(['initial_sync','disabling','removing'].includes(status.stage)||status.stage==='applying'||status.stage==='failed'||status.stage==='disconnected')return 'uncertainPolicyHelp';
 return 'noEnforcement';
}

export function protectionLabel(status:ExtensionStatus,mode:'live'|'demo',hasRules:boolean):TextKey {
 if(mode==='demo')return 'protectionDemo';
 if(['stale','failed','disconnected','configuration_conflict','applying','initial_sync','disabling','removing'].includes(status.stage)||status.busy)return 'protectionUncertain';
 if(status.signed&&status.stage==='applied'&&status.instanceId!==null&&status.providerRevision!==null)return 'protectionConfirmed';
 if(status.signed&&status.stage==='ready')return 'protectionEnabled';
 return hasRules?'protectionSaved':'protectionObserved';
}
