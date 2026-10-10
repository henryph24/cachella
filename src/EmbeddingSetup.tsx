import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import ManagedRuntime, { type Runtime } from './ManagedRuntime';
import { cancelOperation, type Operation } from './operation';

type Progress = { indexed: number; total: number };
export default function EmbeddingSetup({ disabled, singleResident = false }: { disabled: boolean; singleResident?: boolean }) {
  const [ready, setReady] = useState(false);
  const [autoResume, setAutoResume] = useState(false);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const active = useRef(true);
  const running = useRef(false);
  const pause = useRef(false);
  const activeOperation = useRef<Operation | null>(null);
  const revision = useRef(0);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
      pause.current = true;
      revision.current += 1;
      const operation = activeOperation.current;
      if (operation) void cancelIndex(operation).catch(() => undefined);
    };
  }, []);
  useEffect(() => {
    if (!singleResident) return;
    let activeEffect = true;
    void invoke<Progress>('embedding_progress').then(result => {
      if (activeEffect) setProgress(result);
    }).catch(() => {
      // A first explicit start is still required before on-demand indexing is armed.
    });
    return () => { activeEffect = false; };
  }, [singleResident]);
  function connectionChanged(value: boolean) {
    setReady(value); setProgress(null); setError('');
    const current = ++revision.current;
    if (value || singleResident) invoke<Progress>('embedding_progress').then(result => {
      if (active.current && current === revision.current) setProgress(result);
    }).catch(e => { if (active.current && current === revision.current) setError(String(e)); });
  }
  function runtimeChanged(status: Runtime) {
    setAutoResume(singleResident && !!status.auto_resume);
  }
  async function index() {
    if (running.current) return;
    running.current = true; pause.current = false; revision.current += 1;
    setBusy(true); setError('');
    try {
      do {
        const operation: Operation = { cancelRequested: false };
        activeOperation.current = operation;
        const requestId = await invoke<string>('begin_operation');
        operation.requestId = requestId;
        if (operation.cancelRequested) {
          try { await cancelIndex(operation); } catch { /* Cancellation errors are shown when mounted. */ }
          break;
        }
        let next: Progress | undefined;
        let indexError: unknown;
        try { next = await invoke<Progress>('index_library', { requestId }); }
        catch (e) { indexError = e; }
        if (operation.cancelPromise) {
          try { await operation.cancelPromise; } catch { /* Cancellation errors are shown when mounted. */ }
        }
        if (operation.cancellationAccepted) {
          if (active.current) {
            const refreshed = await invoke<Progress>('embedding_progress');
            if (active.current) setProgress(refreshed);
          }
          break;
        }
        if (indexError) throw indexError;
        if (!active.current) return;
        setProgress(next!);
        activeOperation.current = null;
        if (next!.indexed >= next!.total) break;
      } while (!pause.current);
    } catch (e) { if (active.current) setError(String(e)); }
    finally { activeOperation.current = null; running.current = false; if (active.current) setBusy(false); }
  }
  async function cancelIndex(operation = activeOperation.current): Promise<boolean | undefined> {
    pause.current = true;
    if (!operation) return;
    return cancelOperation(operation, invoke, () => {
      if (active.current) {
        setError('Indexing could not be cancelled because the batch already completed.');
      }
    }, error => {
      if (active.current) {
        setError(`Unable to cancel indexing: ${String(error)}`);
      }
    });
  }
  return <>
    <ManagedRuntime embedding disabled={disabled || busy} singleResident={singleResident} onConnectionChange={connectionChanged} onRuntimeChange={runtimeChanged} />
    <section className="settings-card">
      <h2>Semantic index</h2>
      <p>Index imported text and media locally, then enable semantic search beside the search box. Completed passages are saved individually; indexing resumes after interruption.</p>
      <p role="status">{progress ? `${progress.indexed} of ${progress.total} passages indexed` : singleResident ? 'Start the embedding model once to enable on-demand indexing.' : 'Start the embedding model to check the index.'}</p>
      <div className="button-row"><button className="primary" disabled={disabled || busy || (!ready && !autoResume)} onClick={() => void index()}>Index library</button>{busy && <button className="secondary" onClick={() => void cancelIndex().catch(() => undefined)}>Cancel indexing</button>}</div>
      {busy && <p>Indexing up to eight passages per batch. Each request has a 120-second limit.</p>}
      {error && <p role="alert" className="alert error">{error}</p>}
      <p className="format-note">Text indexing uses the first 1,500 UTF-8 bytes of each passage and up to 200 title bytes. Media indexing uses resized images, sampled video frames and 10-second audio segments. Select Gemma 4 E2B in the answer model setup to answer from retrieved media. Qwen uses text sources. Open citations to inspect the original evidence.</p>
    </section>
  </>;
}
