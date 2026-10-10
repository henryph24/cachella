import { afterEach, expect, it, vi } from 'vitest';
import { act, cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import MediaSetup from './MediaSetup';
const bridge = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: bridge.invoke }));
const included = { executable: '/resources/decoder/ffmpeg', source: 'included' as const, available: true, included_available: true };
const manual = { executable: '/trusted/ffmpeg', source: 'manual' as const, available: true, included_available: true };
afterEach(() => { cleanup(); bridge.invoke.mockReset(); });

it('keeps newly selected FFmpeg when an older settings read finishes late', async () => {
  let old: ((value: unknown) => void) | undefined;
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'media_tools') return new Promise(resolve => { old = resolve; });
    return manual;
  });
  const user = userEvent.setup(); render(<MediaSetup disabled={false} />);
  await user.click(screen.getByRole('button', { name: 'Choose FFmpeg' }));
  await screen.findByText(/Manual decoder selected: \/trusted\/ffmpeg/);
  await act(async () => { old!(included); });
  expect(screen.getByText(/Manual decoder selected: \/trusted\/ffmpeg/)).toBeTruthy();
});

it('shows included status and selects the included decoder', async () => {
  bridge.invoke.mockImplementation(async (command: string) => command === 'media_tools' ? manual : included);
  const user = userEvent.setup(); render(<MediaSetup disabled={false} />);
  await screen.findByText(/Manual decoder selected/);
  await user.click(screen.getByRole('button', { name: 'Use included decoder' }));
  await screen.findByText(/Included decoder selected: \/resources\/decoder\/ffmpeg/);
  expect(bridge.invoke).toHaveBeenCalledWith('use_included_media_tool');
});

it('shows an unavailable included decoder without offering an unavailable selection', async () => {
  bridge.invoke.mockResolvedValue({ executable: null, source: 'included', available: false, included_available: false });
  render(<MediaSetup disabled={false} />);
  await screen.findByText('Included decoder unavailable');
  expect(screen.queryByRole('button', { name: 'Use included decoder' })).toBeNull();
});
