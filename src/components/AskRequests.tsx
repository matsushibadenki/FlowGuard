// パス: src/components/AskRequests.tsx
import {useEffect, useState} from 'react';
import {Check, Minus, Clock, Info} from 'lucide-react';
import type {Snapshot} from '../core/types';
import {answerAsk, native} from '../core/store';
import {identityLabel} from '../core/identity';
import type {T} from './ui';
export function AskRequests({snapshot, t, onError}: {snapshot: Snapshot; t: T; onError: (error: unknown) => void}) {
  const asks = snapshot.asks;
  const hasPending = !!asks?.requests.length;
  const [elapsed, setElapsed] = useState(0);
  useEffect(() => {
    const start = performance.now(); setElapsed(0);
    if (!hasPending) return;
    const timer = setInterval(() => setElapsed((performance.now() - start) / 1000), 250);
    return () => clearInterval(timer);
  }, [asks?.pollRevision, hasPending]);
  if (!asks || (!asks.requests.length && !asks.outcomes.length && !asks.error && !asks.lastAnswer)) return null;
  return <section className="ask-inbox surface" aria-label={t('askInbox')}>
    <div className="section-head"><h2><Clock size={18}/>{t('askInbox')} ({asks.requests.length})</h2>
      <span title={t('askScope')}><Info size={16}/></span></div>
    <p className="ask-scope">{t('askScope')}</p>
    {asks.error && <p role="alert">{t(asks.error === 'ask_answer_unknown' ? 'askAnswerUnknown' : 'askSyncFailed')}</p>}
    {asks.lastAnswer && <p role="status">{t(asks.lastAnswer === 'answered' ? 'askAnswerReceived' : 'askNoLongerPending')}</p>}
    <ul>{asks.requests.map(request => {
      const seconds = Math.max(0, Math.ceil(request.remainingSeconds - elapsed));
      const disabled = !native || snapshot.extension.busy || asks.busyIds.includes(request.requestId);
      return <li key={`${request.instanceId}:${request.requestId}`}><div className="ask-request-copy">
        <strong>{request.app}</strong><small>{identityLabel(request.identity)}</small>
        <code>{request.destination.includes(':') ? `[${request.destination}]` : request.destination}:{request.port}</code>
        <span>{t(request.direction === 'inbound' ? 'inbound' : 'outbound')}</span>
        <span className="ask-deadline"><Clock size={14}/>{t('askRemaining').replace('{n}', String(seconds))} {t('askTimeoutResult').replace('{action}', t(request.timeoutAction))}</span>
      </div><div className="ask-answer-buttons">{(['allow', 'block'] as const).map(action =>
        <button key={action} className="secondary-button" disabled={disabled}
          onClick={() => void answerAsk(request, action).catch(onError)}>
          {action === 'allow' ? <Check size={16}/> : <Minus size={16}/>}{t(action === 'allow' ? 'askAllowOnce' : 'askBlockOnce')}
        </button>)}</div></li>;
    })}</ul>
    {asks.outcomes.length > 0 && <details><summary>{t('askOutcomeHistory')} ({asks.outcomes.length})</summary>
      <p className="ask-scope">{t('askOutcomeHelp')}</p><ul>{asks.outcomes.slice(-5).reverse().map(outcome =>
        <li key={`${outcome.request.instanceId}:${outcome.request.requestId}`}>
          <span>{outcome.request.app} <code>{outcome.request.destination}:{outcome.request.port}</code></span>
          <span>{t('askOutcomeAction').replace('{action}', t(outcome.action))} — {t(outcome.reason === 'answered' ? 'askReasonAnswered' : outcome.reason === 'timed_out' ? 'askReasonTimeout' : outcome.reason === 'provider_stopped' ? 'askReasonStop' : outcome.reason === 'capacity' ? 'askReasonCapacity' : outcome.reason === 'transport_unavailable' ? 'askReasonUnavailable' : 'askReasonPolicy')}</span>
        </li>)}</ul></details>}
  </section>;
}
