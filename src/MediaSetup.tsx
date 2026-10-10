import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

type Tools = {
  executable: string | null;
  source: 'included' | 'manual';
  available: boolean;
  included_available: boolean;
};

export default function MediaSetup({ disabled }: { disabled: boolean }) {
  const [tools, setTools] = useState<Tools | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const revision = useRef(0);
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    const current = revision.current;
    invoke<Tools>('media_tools').then(value => {
      if (mounted.current && current === revision.current) setTools(value);
    }).catch(e => {
      if (mounted.current && current === revision.current) setError(String(e));
    });
    return () => { mounted.current = false; revision.current += 1; };
  }, []);

  async function select(command: 'choose_media_tool' | 'use_included_media_tool') {
    const current = ++revision.current;
    setBusy(true);
    setError('');
    try {
      const value = await invoke<Tools>(command);
      if (mounted.current && current === revision.current) setTools(value);
    } catch (e) {
      if (mounted.current && current === revision.current) setError(String(e));
    } finally {
      if (mounted.current && current === revision.current) setBusy(false);
    }
  }

  const status = !tools
    ? 'Checking decoder status'
    : tools.available
      ? tools.source === 'included' ? 'Included decoder selected' : 'Manual decoder selected'
      : tools.source === 'included' ? 'Included decoder unavailable' : 'Manual decoder unavailable';

  return <section className="settings-card">
    <h2>Media import</h2>
    <p>Media is processed locally before indexing.</p>
    <div className="runtime-file"><div><strong>Media decoder</strong><small>{status}{tools?.executable ? `: ${tools.executable}` : ''}</small></div><div>
      {tools?.included_available && tools.source !== 'included' && <button className="secondary" disabled={disabled || busy} onClick={() => void select('use_included_media_tool')}>Use included decoder</button>}
      <button className="secondary" disabled={disabled || busy} onClick={() => void select('choose_media_tool')}>Choose FFmpeg</button>
    </div></div>
    <p className="format-note">PNG/JPEG, WAV/MP3/FLAC/OGG, MP4/WebM. Up to 32 MB and 2 minutes per media file. Video search samples a frame every 5 seconds and audio every 10 seconds. The stored media limit is 512 MB.</p>
    {tools?.included_available && <p className="format-note">Included FFmpeg 9.0.2 with libx264 is provided with its license and source materials in the decoder folder.</p>}
    {error && <p role="alert" className="alert error">{error}</p>}
  </section>;
}
