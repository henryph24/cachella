import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import MediaPreview from './MediaPreview';
vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: (id: string, scheme: string) => `${scheme}://localhost/${id}` }));
afterEach(cleanup);
it('renders a local image with an accessible source title', () => {
  render(<MediaPreview id={'a'.repeat(64)} kind="image" title="Red circle.png" />);
  const image = screen.getByRole('img', { name: 'Red circle.png' }) as HTMLImageElement;
  expect(image.src).toBe(`cachella-media://localhost/${'a'.repeat(64)}`);
});
it('seeks a video to the retrieved interval without autoplay', () => {
  render(<MediaPreview id={'b'.repeat(64)} kind="video" title="Demo.mp4" startMs={5000} endMs={10000} />);
  const video = screen.getByLabelText('Demo.mp4 media preview') as HTMLVideoElement;
  fireEvent.loadedMetadata(video);
  expect(video.currentTime).toBe(5);
  expect(video.autoplay).toBe(false);
  expect(screen.getByText('Matched interval: 0:05–0:10')).toBeTruthy();
});
it('reports playback errors while preserving the source title', () => {
  render(<MediaPreview id={'c'.repeat(64)} kind="audio" title="Meeting.wav" startMs={10000} endMs={20000} />);
  fireEvent.error(screen.getByLabelText('Meeting.wav media preview'));
  expect(screen.getByRole('alert').textContent).toContain('preview');
});
