import { useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';

export function mediaTime(milliseconds: number) {
  const seconds = Math.max(0, Math.floor(milliseconds / 1000));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
}

export default function MediaPreview({ id, title, kind, startMs, endMs }: { id: string; title: string; kind: string; startMs?: number | null; endMs?: number | null }) {
  const [error, setError] = useState(false);
  const url = convertFileSrc(id, 'cachella-media');
  const seek = (element: HTMLMediaElement) => {
    if (startMs != null && Number.isFinite(startMs)) element.currentTime = Math.max(0, startMs / 1000);
  };
  return <div className="media-preview">
    {kind === 'image' ? <img src={url} alt={title} onError={() => setError(true)} /> : kind === 'video' ?
      <video src={url} controls preload="metadata" aria-label={`${title} media preview`} onLoadedMetadata={e => seek(e.currentTarget)} onError={() => setError(true)} /> :
      <audio src={url} controls preload="metadata" aria-label={`${title} media preview`} onLoadedMetadata={e => seek(e.currentTarget)} onError={() => setError(true)} />}
    {startMs != null && <p>Matched interval: {mediaTime(startMs)}{endMs != null ? `–${mediaTime(endMs)}` : ''}</p>}
    {error && <p role="alert" className="alert error">This media preview could not be loaded. Reopen the source or import the file again.</p>}
    <p className="format-note">Stored local preview. Images are resized; audio and video are converted for playback. The original file stays in its selected location.</p>
  </div>;
}
