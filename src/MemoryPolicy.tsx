import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export type MemoryPolicy = 'both' | 'single';

export default function MemoryPolicyControl({ disabled, onChange }: { disabled: boolean; onChange: (policy: MemoryPolicy) => void }) {
  const [policy, setPolicy] = useState<MemoryPolicy | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const mounted = useRef(true);
  const savingNow = useRef(false);

  useEffect(() => {
    let active = true;
    mounted.current = true;
    void invoke<MemoryPolicy>('memory_policy').then(value => {
      if (value !== 'both' && value !== 'single') throw Error('The runtime returned an invalid memory policy.');
      if (!active || !mounted.current) return;
      setPolicy(value); onChange(value);
    }).catch(e => { if (active && mounted.current) setError(`Could not load managed model memory settings: ${String(e)}`); });
    return () => { active = false; mounted.current = false; };
  }, [onChange]);

  async function change(single: boolean) {
    if (!policy || savingNow.current) return;
    const next: MemoryPolicy = single ? 'single' : 'both';
    savingNow.current = true; setSaving(true); setError('');
    try {
      const saved = await invoke<MemoryPolicy>('set_memory_policy', { policy: next });
      if (saved !== 'both' && saved !== 'single') throw Error('The runtime returned an invalid memory policy.');
      if (mounted.current) setPolicy(saved);
      onChange(saved);
    } catch (e) { if (mounted.current) setError(String(e)); }
    finally { savingNow.current = false; if (mounted.current) setSaving(false); }
  }

  return <section className="settings-card">
    <div className="settings-title"><h2>Managed model memory</h2><span className="status-text">{policy === null ? 'Unavailable' : policy === 'single' ? 'One model' : 'Both models'}</span></div>
    <div className="toggle-row"><div><strong>Keep one managed model loaded</strong><p>Switches between semantic search and answers as needed. Each switch adds startup time and clears that model’s prompt cache.</p></div><label className="switch"><input type="checkbox" checked={policy === 'single'} disabled={disabled || saving || policy === null} onChange={e => void change(e.target.checked)} aria-label="Keep one managed model loaded" /><span /></label></div>
    {policy === 'single' && <p role="status">One managed model is kept loaded at a time. Start each model once this session to allow on-demand switching.</p>}
    {error && <p className="alert error" role="alert">{error}</p>}
    <p className="format-note">This applies only to Cachella-managed engines. Independently managed loopback servers are excluded. It limits managed model residency; actual memory use still depends on the model, context, media input and your operating system.</p>
  </section>;
}
