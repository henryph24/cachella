import { afterEach, expect, it, vi } from 'vitest';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import EmbeddingSetup from './EmbeddingSetup';
const bridge = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: bridge.invoke }));
afterEach(() => { cleanup(); bridge.invoke.mockReset(); });
const ready = { config: { executable: '/llama-server', model: '/embed.gguf', projector: '/project.gguf' }, running: true, ready: true };
it('allows indexing with an armed on-demand embedding model', async () => {
  const suspended = { ...ready, running: false, ready: false, auto_resume: true };
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'bundled_engine') return null;
    if (command === 'model_catalog') return [];
    if (command === 'model_install_status') return { phase: 'idle', bundle_id: null, downloaded_bytes: 0, total_bytes: 0, current_file: null, error: null };
    if (command === 'managed_runtime') return suspended;
    if (command === 'embedding_progress') return { indexed: 2, total: 3 };
    if (command === 'begin_operation') return 'index-on-demand';
    if (command === 'index_library') return { indexed: 3, total: 3 };
    throw Error(command);
  });
  const user = userEvent.setup(); render(<EmbeddingSetup disabled={false} singleResident />);
  await screen.findByText('On demand');
  const index = screen.getByRole('button', { name: 'Index library' });
  expect((index as HTMLButtonElement).disabled).toBe(false);
  await user.click(index);
  await screen.findByText('3 of 3 passages indexed');
  expect(bridge.invoke).toHaveBeenCalledWith('index_library', { requestId: 'index-on-demand' });
});
it('cancels an active batch, refreshes committed passages, and resumes indexing', async () => {
  let finish: ((value: { indexed: number; total: number }) => void) | undefined;
  let cancelled = false;
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'bundled_engine') return null;
    if (command === 'model_catalog') return [];
    if (command === 'model_install_status') return { phase: 'idle', bundle_id: null, downloaded_bytes: 0, total_bytes: 0, current_file: null, error: null };
    if (command === 'managed_runtime') return ready;
    if (command === 'embedding_progress') return { indexed: cancelled ? 10 : 2, total: 12 };
    if (command === 'begin_operation') return 'index-1';
    if (command === 'index_library') return new Promise(resolve => { finish = resolve; });
    if (command === 'cancel_operation') { cancelled = true; return true; }
    throw Error(command);
  });
  const user = userEvent.setup(); render(<EmbeddingSetup disabled={false} />);
  await screen.findByText('2 of 12 passages indexed');
  await user.click(screen.getByRole('button', { name: 'Index library' }));
  await screen.findByRole('button', { name: 'Cancel indexing' });
  await user.click(screen.getByRole('button', { name: 'Cancel indexing' }));
  await waitFor(() => expect(bridge.invoke).toHaveBeenCalledWith('cancel_operation', { requestId: 'index-1' }));
  await act(async () => { finish!({ indexed: 10, total: 12 }); });
  await screen.findByText('10 of 12 passages indexed');
  expect(bridge.invoke.mock.calls.filter(([cmd]) => cmd === 'index_library')).toHaveLength(1);
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'bundled_engine') return null;
    if (command === 'model_catalog') return [];
    if (command === 'model_install_status') return { phase: 'idle', bundle_id: null, downloaded_bytes: 0, total_bytes: 0, current_file: null, error: null };
    if (command === 'managed_runtime') return ready;
    if (command === 'embedding_progress') return { indexed: 10, total: 12 };
    if (command === 'begin_operation') return 'index-2';
    if (command === 'index_library') return new Promise(resolve => { finish = resolve; });
    throw Error(command);
  });
  await user.click(screen.getByRole('button', { name: 'Index library' }));
  await act(async () => { finish!({ indexed: 12, total: 12 }); });
  await screen.findByText('12 of 12 passages indexed');
  expect(bridge.invoke.mock.calls.filter(([cmd]) => cmd === 'index_library')).toHaveLength(2);
  expect(screen.queryByRole('button', { name: 'Cancel indexing' })).toBeNull();
});
it('stops scheduling batches after unmount and reports engine failures', async () => {
  let finish: ((value: { indexed: number; total: number }) => void) | undefined;
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'bundled_engine') return null;
    if (command === 'model_catalog') return [];
    if (command === 'model_install_status') return { phase: 'idle', bundle_id: null, downloaded_bytes: 0, total_bytes: 0, current_file: null, error: null };
    if (command === 'managed_runtime') return ready;
    if (command === 'embedding_progress') return { indexed: 0, total: 20 };
    if (command === 'begin_operation') return 'index-unmount';
    if (command === 'index_library') return new Promise(resolve => { finish = resolve; });
    if (command === 'cancel_operation') return true;
    throw Error(command);
  });
  const user = userEvent.setup(); const view = render(<EmbeddingSetup disabled={false} />);
  await screen.findByText('0 of 20 passages indexed');
  await user.click(screen.getByRole('button', { name: 'Index library' }));
  await screen.findByRole('button', { name: 'Cancel indexing' });
  view.unmount();
  await act(async () => { finish!({ indexed: 8, total: 20 }); });
  expect(bridge.invoke.mock.calls.filter(([cmd]) => cmd === 'index_library')).toHaveLength(1);
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'bundled_engine') return null;
    if (command === 'model_catalog') return [];
    if (command === 'model_install_status') return { phase: 'idle', bundle_id: null, downloaded_bytes: 0, total_bytes: 0, current_file: null, error: null };
    if (command === 'managed_runtime') return ready;
    if (command === 'embedding_progress') return { indexed: 8, total: 20 };
    throw Error('Engine exited');
  });
  render(<EmbeddingSetup disabled={false} />);
  await screen.findByText('8 of 20 passages indexed');
  await user.click(screen.getByRole('button', { name: 'Index library' }));
  await screen.findByRole('alert');
  await waitFor(() => expect((screen.getByRole('button', { name: 'Index library' }) as HTMLButtonElement).disabled).toBe(false));
});

it('cancels a reservation that arrives after indexing was stopped and never indexes', async () => {
  let resolveBegin: ((value: string) => void) | undefined;
  bridge.invoke.mockImplementation((command: string) => {
    if (command === 'bundled_engine') return Promise.resolve(null);
    if (command === 'model_catalog') return Promise.resolve([]);
    if (command === 'model_install_status') return Promise.resolve({ phase: 'idle', bundle_id: null, downloaded_bytes: 0, total_bytes: 0, current_file: null, error: null });
    if (command === 'managed_runtime') return Promise.resolve(ready);
    if (command === 'embedding_progress') return Promise.resolve({ indexed: 0, total: 12 });
    if (command === 'begin_operation') return new Promise<string>(resolve => { resolveBegin = resolve; });
    if (command === 'cancel_operation') return Promise.reject(new Error('Bridge unavailable'));
    if (command === 'index_library') throw new Error('index_library must not run after cancellation');
    throw Error(command);
  });
  const user = userEvent.setup(); render(<EmbeddingSetup disabled={false} />);
  await screen.findByText('0 of 12 passages indexed');
  await user.click(screen.getByRole('button', { name: 'Index library' }));
  await user.click(screen.getByRole('button', { name: 'Cancel indexing' }));
  await act(async () => { resolveBegin!('index-pending'); });
  await screen.findByText(/Unable to cancel indexing/);
  expect(bridge.invoke).toHaveBeenCalledWith('cancel_operation', { requestId: 'index-pending' });
  expect(bridge.invoke.mock.calls.filter(([command]) => command === 'index_library')).toHaveLength(0);
});

it('keeps committed batch progress and stops future batches when cancellation is declined', async () => {
  let finish: ((value: { indexed: number; total: number }) => void) | undefined;
  let resolveCancel: ((value: boolean) => void) | undefined;
  bridge.invoke.mockImplementation((command: string) => {
    if (command === 'bundled_engine') return Promise.resolve(null);
    if (command === 'model_catalog') return Promise.resolve([]);
    if (command === 'model_install_status') return Promise.resolve({ phase: 'idle', bundle_id: null, downloaded_bytes: 0, total_bytes: 0, current_file: null, error: null });
    if (command === 'managed_runtime') return Promise.resolve(ready);
    if (command === 'embedding_progress') return Promise.resolve({ indexed: 0, total: 16 });
    if (command === 'begin_operation') return Promise.resolve('index-declined');
    if (command === 'index_library') return new Promise(resolve => { finish = resolve; });
    if (command === 'cancel_operation') return new Promise<boolean>(resolve => { resolveCancel = resolve; });
    throw Error(command);
  });
  const user = userEvent.setup(); render(<EmbeddingSetup disabled={false} />);
  await screen.findByText('0 of 16 passages indexed');
  await user.click(screen.getByRole('button', { name: 'Index library' }));
  await user.click(screen.getByRole('button', { name: 'Cancel indexing' }));
  await act(async () => { finish!({ indexed: 8, total: 16 }); });
  expect(screen.queryByText('8 of 16 passages indexed')).toBeNull();
  await act(async () => { resolveCancel!(false); });
  await screen.findByText('8 of 16 passages indexed');
  expect(bridge.invoke.mock.calls.filter(([command]) => command === 'index_library')).toHaveLength(1);
  expect(screen.getByText(/could not be cancelled because the batch already completed/)).toBeTruthy();
});

it('keeps committed batch progress and stops future batches when cancellation rejects', async () => {
  let finish: ((value: { indexed: number; total: number }) => void) | undefined;
  let rejectCancel: ((reason?: unknown) => void) | undefined;
  bridge.invoke.mockImplementation((command: string) => {
    if (command === 'bundled_engine') return Promise.resolve(null);
    if (command === 'model_catalog') return Promise.resolve([]);
    if (command === 'model_install_status') return Promise.resolve({ phase: 'idle', bundle_id: null, downloaded_bytes: 0, total_bytes: 0, current_file: null, error: null });
    if (command === 'managed_runtime') return Promise.resolve(ready);
    if (command === 'embedding_progress') return Promise.resolve({ indexed: 0, total: 16 });
    if (command === 'begin_operation') return Promise.resolve('index-rejected');
    if (command === 'index_library') return new Promise(resolve => { finish = resolve; });
    if (command === 'cancel_operation') return new Promise((_resolve, reject) => { rejectCancel = reject; });
    throw Error(command);
  });
  const user = userEvent.setup(); render(<EmbeddingSetup disabled={false} />);
  await screen.findByText('0 of 16 passages indexed');
  await user.click(screen.getByRole('button', { name: 'Index library' }));
  await user.click(screen.getByRole('button', { name: 'Cancel indexing' }));
  await act(async () => { finish!({ indexed: 8, total: 16 }); });
  await act(async () => { rejectCancel!(new Error('Bridge unavailable')); });
  await screen.findByText('8 of 16 passages indexed');
  await screen.findByText(/Unable to cancel indexing: Error: Bridge unavailable/);
  expect(bridge.invoke.mock.calls.filter(([command]) => command === 'index_library')).toHaveLength(1);
});

it('cancels a pending reservation on unmount without indexing', async () => {
  let resolveBegin: ((value: string) => void) | undefined;
  bridge.invoke.mockImplementation((command: string) => {
    if (command === 'bundled_engine') return Promise.resolve(null);
    if (command === 'model_catalog') return Promise.resolve([]);
    if (command === 'model_install_status') return Promise.resolve({ phase: 'idle', bundle_id: null, downloaded_bytes: 0, total_bytes: 0, current_file: null, error: null });
    if (command === 'managed_runtime') return Promise.resolve(ready);
    if (command === 'embedding_progress') return Promise.resolve({ indexed: 0, total: 12 });
    if (command === 'begin_operation') return new Promise<string>(resolve => { resolveBegin = resolve; });
    if (command === 'cancel_operation') return Promise.resolve(true);
    if (command === 'index_library') throw new Error('index_library must not run after unmount');
    throw Error(command);
  });
  const user = userEvent.setup(); const view = render(<EmbeddingSetup disabled={false} />);
  await screen.findByText('0 of 12 passages indexed');
  await user.click(screen.getByRole('button', { name: 'Index library' }));
  view.unmount();
  await act(async () => { resolveBegin!('index-unmounted'); });
  await waitFor(() => expect(bridge.invoke).toHaveBeenCalledWith('cancel_operation', { requestId: 'index-unmounted' }));
  expect(bridge.invoke.mock.calls.filter(([command]) => command === 'index_library')).toHaveLength(0);
});
