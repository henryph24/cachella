import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import ModelDownload from './ModelDownload';
import { Cpu, FolderOpen, LoaderCircle, Play, Square } from 'lucide-react';

export type Runtime = { config: { executable: string | null; model: string | null; projector?: string | null; bundled_engine?: boolean }; running: boolean; ready: boolean; managed: boolean; endpoint: string | null; auto_resume?: boolean };

export default function ManagedRuntime({ disabled, onConnectionChange, onRuntimeChange, embedding = false, singleResident = false }: { embedding?: boolean; disabled: boolean; onConnectionChange: (running: boolean) => void; onRuntimeChange?: (runtime: Runtime) => void; singleResident?: boolean }) {
  const [runtime, setRuntime] = useState<Runtime | null>(null);
  const [bundledEngine, setBundledEngine] = useState<string | null>(null);
  const [busy, setBusy] = useState('');
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState('');
  const priorReady = useRef(false);
  const mounted = useRef(true);
  const revision = useRef(0);
  const operation = useRef(false);
  const polling = useRef(false);
  const notify = useRef(onConnectionChange);
  notify.current = onConnectionChange;
  useEffect(() => {
    let active = true; mounted.current = true;
    async function refresh() {
      if (operation.current || installing || polling.current) return;
      const requestedRevision = revision.current;
      polling.current = true;
      try {
        const status = await invoke<Runtime>('managed_runtime', embedding ? { embedding: true } : undefined);
        if (!active || requestedRevision !== revision.current) return;
        setRuntime(status);
        onRuntimeChange?.(status);
        if (priorReady.current !== status.ready) notify.current(status.ready);
        priorReady.current = status.ready;
      } catch (e) { if (active && requestedRevision === revision.current) setError(String(e)); }
      finally { polling.current = false; }
    }
    void refresh();
    const timer = setInterval(() => { void refresh(); }, 3000);
    return () => { active = false; mounted.current = false; revision.current += 1; clearInterval(timer); };
  }, [embedding, installing, singleResident]);
  useEffect(() => {
    let active = true;
    void invoke<string | null>('bundled_engine').then(path => {
      if (active) setBundledEngine(typeof path === 'string' && path.length > 0 ? path : null);
    }).catch(e => {
      if (active) setError(`Could not check for the included engine: ${String(e)}`);
    });
    return () => { active = false; };
  }, []);
  async function run(command: string, kind?: string) {
    if (operation.current) return;
    operation.current = true; const operationRevision = ++revision.current;
    setBusy(command); setError('');
    try {
      const args = { ...(kind ? { kind } : {}), ...(embedding ? { embedding: true } : {}) };
      const status = await invoke<Runtime>(command, Object.keys(args).length ? args : undefined);
      if (!mounted.current || operationRevision !== revision.current) return;
      setRuntime(status); priorReady.current = status.ready;
      onRuntimeChange?.(status);
      if (command === 'start_runtime' || command === 'stop_runtime') onConnectionChange(status.ready);
    } catch (e) { if (mounted.current) setError(String(e)); }
    finally { operation.current = false; if (mounted.current) setBusy(''); }
  }
  const onDemand = singleResident && !!runtime?.auto_resume && !runtime.running;
  const canStop = !!runtime?.running || (singleResident && !!runtime?.auto_resume);
  return <section className="settings-card">
    <div className="settings-title"><Cpu size={19} /><h2>{embedding ? 'Semantic search model' : 'Run a model with Cachella'}</h2><span className="status-text">{runtime?.ready ? 'Running' : runtime?.running ? 'Loading' : onDemand ? 'On demand' : 'Stopped'}</span></div>
    <p>{embedding ? 'Select an EmbeddingGemma 2 compatible llama-server, the text GGUF and its matching vision/audio projector. This separate local process powers semantic search.' : 'Select llama-server from your trusted llama.cpp installation and a compatible GGUF model. Cachella starts it privately on this computer and stops it when you quit.'}</p>
    <div className="runtime-file"><div><strong>Inference engine</strong><small>{runtime?.config.executable ?? 'No executable selected'}</small></div><div className="button-row">{bundledEngine && bundledEngine !== runtime?.config.executable && <button className="secondary" disabled={disabled || installing || !!busy || !!runtime?.running} onClick={() => void run('use_bundled_engine')}><Cpu size={15} />Use included engine</button>}<button className="secondary" disabled={disabled || installing || !!busy || runtime?.running} onClick={() => void run('choose_runtime_file', 'executable')}><FolderOpen size={15} />Choose engine</button></div></div>
    <div className="runtime-file"><div><strong>Model file</strong><small>{runtime?.config.model ?? 'No GGUF model selected'}</small></div><button className="secondary" disabled={disabled || installing || !!busy || runtime?.running} onClick={() => void run('choose_runtime_file', 'model')}><FolderOpen size={15} />Choose model</button></div>
    {embedding && <div className="runtime-file"><div><strong>Vision/audio projector</strong><small>{runtime?.config.projector ?? 'No projector selected'}</small></div><button className="secondary" disabled={disabled || installing || !!busy || runtime?.running} onClick={() => void run('choose_runtime_file', 'projector')}><FolderOpen size={15} />Choose projector</button></div>}
    {!embedding && runtime?.config.projector && <div className="runtime-file"><div><strong>Vision/audio projector</strong><small>{runtime.config.projector}</small><small>Media answers require this matching vision/audio projector.</small></div></div>}
    <ModelDownload embedding={embedding} disabled={disabled || !!busy || !!runtime?.running} onBusyChange={setInstalling} onInstalled={status => {
      revision.current += 1;
      setRuntime(status); priorReady.current = status.ready;
      onRuntimeChange?.(status);
      notify.current(status.ready);
    }} />
    <div className="button-row"><button className="primary" disabled={disabled || installing || !!busy || !runtime?.config.executable || !runtime?.config.model || (embedding && !runtime?.config.projector) || runtime?.running} onClick={() => void run('start_runtime')}>{busy === 'start_runtime' ? <LoaderCircle size={16} className="spin" /> : <Play size={16} />}{embedding ? 'Start embedding model' : 'Start model'}</button><button className="secondary" disabled={disabled || installing || !!busy || !canStop} onClick={() => void run('stop_runtime')}><Square size={15} />{embedding ? 'Stop embedding model' : 'Stop model'}</button></div>
    {busy === 'start_runtime' && <p role="status">Loading your model. Startup may take up to 60 seconds.</p>}
    {onDemand && <p role="status">This model is armed for this session and starts on demand. Stop disables automatic startup.</p>}
    {error && <p className="alert error" role="alert">{error}</p>}
    <p className="format-note">Packaged builds include a compatible engine. You can also choose an existing trusted llama-server installation and model files; selected files stay in their original locations.</p>
  </section>;
}
