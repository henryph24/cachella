import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Download, LoaderCircle } from 'lucide-react';
import type { Runtime } from './ManagedRuntime';

type Bundle = { id: string; title: string; description: string; license: string; publisher: string; source_url: string; total_bytes: number; embedding: boolean };
type InstallStatus = { bundle_id: string | null; phase: 'idle' | 'downloading' | 'verifying' | 'ready' | 'cancelled' | 'error'; downloaded_bytes: number; total_bytes: number; current_file: string | null; error: string | null };
let status: InstallStatus | null = null;
let revision = 0;
let pending = false;
let readSequence = 0;
const listeners = new Set<() => void>();
function publish(next: InstallStatus) { status = next; for (const listener of listeners) listener(); }
function subscribe(listener: () => void) { listeners.add(listener); return () => { listeners.delete(listener); }; }
function active(value: InstallStatus | null) { return value?.phase === 'downloading' || value?.phase === 'verifying'; }
function size(bytes: number) { return `${(bytes / 1073741824).toFixed(2)} GiB`; }

export default function ModelDownload({ embedding, disabled, onInstalled, onBusyChange }: { embedding: boolean; disabled: boolean; onInstalled: (runtime: Runtime) => void; onBusyChange: (busy: boolean) => void }) {
  const current = useSyncExternalStore(subscribe, () => status);
  const [bundles, setBundles] = useState<Bundle[]>([]);
  const [bundleId, setBundleId] = useState('');
  const [error, setError] = useState('');
  const [cancelling, setCancelling] = useState(false);
  const mounted = useRef(false);
  const manualSelection = useRef(false);
  const callbacks = useRef({ onInstalled, onBusyChange });
  callbacks.current = { onInstalled, onBusyChange };
  const selectedBundle = bundles.find(item => item.id === bundleId) ?? null;
  // Status is shared across settings remounts and identifies the most recent
  // installer bundle before the user makes a new selection.
  const activeBundle = active(current) ? bundles.find(item => item.id === current?.bundle_id) ?? null : null;
  const bundle = activeBundle ?? selectedBundle;
  const own = !!bundle && current?.bundle_id === bundle.id;
  const ownBusy = own && (active(current) || pending);
  const globallyBusy = active(current) || pending;
  useEffect(() => { callbacks.current.onBusyChange(globallyBusy); }, [globallyBusy]);
  useEffect(() => {
    if (manualSelection.current || !current?.bundle_id || !bundles.some(item => item.id === current.bundle_id)) return;
    setBundleId(current.bundle_id);
  }, [bundles, current?.bundle_id]);
  useEffect(() => {
    let alive = true; let reading = false; mounted.current = true;
    void invoke<Bundle[]>('model_catalog').then(catalog => {
      if (!alive || !Array.isArray(catalog)) return;
      const candidates = catalog.filter(item => item.embedding === embedding);
      setBundles(candidates);
      setBundleId(current => {
        if (candidates.some(item => item.id === current)) return current;
        return (!embedding ? candidates.find(item => item.id === 'qwen3-1.7b-q4') : candidates[0])?.id ?? '';
      });
    }).catch(e => { if (alive) setError(String(e)); });
    async function refresh() {
      if (reading) return;
      reading = true; const requestedRevision = revision; const requestedRead = ++readSequence;
      try {
        const next = await invoke<InstallStatus>('model_install_status');
        if (alive && requestedRevision === revision && requestedRead === readSequence && next?.phase && !(pending && next.phase === 'idle')) publish(next);
      } catch (e) { if (alive && requestedRevision === revision) setError(String(e)); }
      finally { reading = false; }
    }
    void refresh();
    const timer = setInterval(() => { void refresh(); }, 1000);
    return () => { alive = false; mounted.current = false; clearInterval(timer); };
  }, [embedding]);
  async function install() {
    if (!bundle || disabled || pending || active(status) || !status) return;
    pending = true; revision += 1;
    setError(''); setCancelling(false);
    publish({ bundle_id: bundle.id, phase: 'downloading', downloaded_bytes: 0, total_bytes: bundle.total_bytes, current_file: null, error: null });
    callbacks.current.onBusyChange(true);
    try {
      const runtime = await invoke<Runtime>('install_model', { bundleId: bundle.id });
      revision += 1;
      publish({ bundle_id: bundle.id, phase: 'ready', downloaded_bytes: bundle.total_bytes, total_bytes: bundle.total_bytes, current_file: null, error: null });
      if (mounted.current) callbacks.current.onInstalled(runtime);
    } catch (e) {
      revision += 1;
      // The native status distinguishes cancellation from an install failure.
      try {
        const latest = await invoke<InstallStatus>('model_install_status');
        if (mounted.current) setError(latest?.phase === 'cancelled' ? '' : String(e));
        publish(latest?.phase === 'cancelled' ? latest : { bundle_id: bundle.id, phase: 'error', downloaded_bytes: status?.downloaded_bytes ?? 0, total_bytes: bundle.total_bytes, current_file: null, error: String(e) });
      } catch {
        if (mounted.current) setError(String(e));
        publish({ bundle_id: bundle.id, phase: 'error', downloaded_bytes: 0, total_bytes: bundle.total_bytes, current_file: null, error: String(e) });
      }
    } finally { pending = false; revision += 1; if (status) publish({ ...status }); if (mounted.current) { setCancelling(false); callbacks.current.onBusyChange(false); } }
  }
  async function cancel() {
    setCancelling(true); setError('');
    try { await invoke('cancel_model_install'); }
    catch (e) { if (mounted.current) { setError(String(e)); setCancelling(false); } }
  }
  const downloadError = error || (own && current?.phase === 'error' ? current.error : '');
  return <div>
    {!bundle && active(current) && <div role="status"><p>{current?.phase === 'verifying' ? 'Verifying downloaded files' : 'Downloading model'}: {size(current?.downloaded_bytes ?? 0)} of {size(current?.total_bytes ?? 0)}</p>{current?.current_file && <p className="format-note">{current.current_file}</p>}<progress aria-label="Model download progress" max={current?.total_bytes || 1} value={current?.downloaded_bytes ?? 0} /><div className="button-row"><button className="secondary" disabled={cancelling} onClick={() => void cancel()}><LoaderCircle size={15} className="spin" />{cancelling ? 'Cancelling download' : 'Cancel download'}</button></div></div>}
    {bundle && <>
      {!embedding && bundles.length > 1 && <label>Answer model<select aria-label="Answer model" value={bundle.id} disabled={disabled || pending || active(current)} onChange={event => { manualSelection.current = true; setBundleId(event.target.value); setError(''); }}>{bundles.map(item => <option key={item.id} value={item.id}>{item.title} · {size(item.total_bytes)}</option>)}</select></label>}
      <div className="runtime-file"><div><strong>{bundle.title}</strong><small>{bundle.description}</small><small>{size(bundle.total_bytes)} download · {bundle.publisher} · {bundle.license}</small></div><button className="secondary" disabled={disabled || !current || pending || active(current)} onClick={() => void install()}><Download size={15} />Download and select model</button></div>
      <p className="format-note">Clicking Download and select model downloads the whole model from Hugging Face and its delivery network to this computer. Cachella verifies the files before selecting them. Use the included engine or choose a trusted llama-server installation.</p>
      {ownBusy && <div role="status"><p>{current?.phase === 'verifying' ? 'Verifying downloaded files' : 'Downloading model'}: {size(current?.downloaded_bytes ?? 0)} of {size(current?.total_bytes ?? bundle.total_bytes)}</p>{current?.current_file && <p className="format-note">{current.current_file}</p>}<progress aria-label={`${bundle.title} download progress`} max={current?.total_bytes || bundle.total_bytes} value={current?.downloaded_bytes ?? 0} /><div className="button-row"><button className="secondary" disabled={cancelling} onClick={() => void cancel()}><LoaderCircle size={15} className="spin" />{cancelling ? 'Cancelling download' : 'Cancel download'}</button></div></div>}
      {active(current) && !own && <p className="format-note">Another model download is in progress.</p>}
      {own && current?.phase === 'ready' && <p className="format-note" role="status">Download verified. Model files are saved on this computer.</p>}
      {own && current?.phase === 'cancelled' && <p className="format-note" role="status">Download cancelled.</p>}
      <p className="format-note">Leaving this page continues the download; quitting Cachella cancels it. Interrupted files restart on retry; completed files are verified and reused.</p>
    </>}
    {downloadError && <p className="alert error" role="alert">{downloadError}</p>}
  </div>;
}
