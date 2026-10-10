import { afterEach, expect, it, vi } from 'vitest';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import ModelDownload from './ModelDownload';
const bridge = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: bridge.invoke }));
afterEach(() => { cleanup(); bridge.invoke.mockReset(); });
const bundle = { id: 'qwen3-1.7b-q4', title: 'Qwen3 1.7B', description: 'Local answers', license: 'Apache-2.0', publisher: 'Qwen', source_url: 'https://huggingface.co/Qwen', total_bytes: 1073741824, embedding: false };
const multimodalBundle = { id: 'gemma4-e2b-q4', title: 'Gemma 4 E2B', description: 'Vision and audio answers', license: 'Apache-2.0', publisher: 'ggml-org', source_url: 'https://huggingface.co/ggml-org/gemma-4-E2B-it-GGUF', total_bytes: 3398849248, embedding: false };
const idle = { bundle_id: null, phase: 'idle', downloaded_bytes: 0, total_bytes: 0, current_file: null, error: null };
const runtime = { config: { executable: '/engine', model: '/download/model.gguf' }, running: false, ready: false, managed: false, endpoint: null };
function mock(finish?: () => Promise<unknown>) {
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'model_catalog') return [bundle, multimodalBundle, { ...bundle, id: 'embeddinggemma2-q8', title: 'EmbeddingGemma 2', embedding: true }];
    if (command === 'model_install_status') return idle;
    if (command === 'install_model') return finish ? finish() : runtime;
    if (command === 'cancel_model_install') return undefined;
    throw Error(command);
  });
}
it('downloads only after an explicit click and propagates the selected configuration', async () => {
  mock(); const installed = vi.fn(); const user = userEvent.setup();
  render(<ModelDownload embedding={false} disabled={false} onInstalled={installed} onBusyChange={vi.fn()} />);
  await screen.findByText('Qwen3 1.7B');
  expect(bridge.invoke.mock.calls.some(([command]) => command === 'install_model')).toBe(false);
  expect(screen.getByText(/Apache-2.0/)).toBeTruthy();
  await user.click(screen.getByRole('button', { name: 'Download and select model' }));
  await waitFor(() => expect(installed).toHaveBeenCalledWith(runtime));
  expect(bridge.invoke).toHaveBeenCalledWith('install_model', { bundleId: bundle.id });
  expect(screen.getByText(/Download verified/)).toBeTruthy();
});
it('keeps Qwen selected by default and installs the chosen generation bundle', async () => {
  mock(); const user = userEvent.setup();
  render(<ModelDownload embedding={false} disabled={false} onInstalled={vi.fn()} onBusyChange={vi.fn()} />);
  const select = await screen.findByLabelText('Answer model');
  expect((select as HTMLSelectElement).value).toBe(bundle.id);
  await user.selectOptions(select, multimodalBundle.id);
  expect(screen.getByText('Gemma 4 E2B')).toBeTruthy();
  expect(screen.getByText(/3.17 GiB download/)).toBeTruthy();
  await user.click(screen.getByRole('button', { name: 'Download and select model' }));
  expect(bridge.invoke).toHaveBeenCalledWith('install_model', { bundleId: multimodalBundle.id });
});
it('keeps an active Gemma download cancellable after remount when status arrives before the catalog', async () => {
  let finish!: (value: unknown) => void;
  let resolveCatalog!: (value: unknown) => void;
  let remounted = false;
  const activeGemma = { bundle_id: multimodalBundle.id, phase: 'downloading' as const, downloaded_bytes: 12, total_bytes: multimodalBundle.total_bytes, current_file: 'gemma-4-E2B-it-Q4_0.gguf', error: null };
  bridge.invoke.mockImplementation((command: string) => {
    if (command === 'model_catalog') return remounted ? new Promise(resolve => { resolveCatalog = resolve; }) : Promise.resolve([bundle, multimodalBundle]);
    if (command === 'model_install_status') return Promise.resolve(remounted ? activeGemma : idle);
    if (command === 'install_model') return new Promise(resolve => { finish = resolve; });
    if (command === 'cancel_model_install') return Promise.resolve();
    throw Error(command);
  });
  const user = userEvent.setup();
  const first = render(<ModelDownload embedding={false} disabled={false} onInstalled={vi.fn()} onBusyChange={vi.fn()} />);
  const select = await screen.findByLabelText('Answer model');
  await user.selectOptions(select, multimodalBundle.id);
  await user.click(screen.getByRole('button', { name: 'Download and select model' }));
  first.unmount();
  remounted = true;
  const busy = vi.fn();
  render(<ModelDownload embedding={false} disabled={false} onInstalled={vi.fn()} onBusyChange={busy} />);
  await screen.findByText(/Downloading model/);
  expect(busy).toHaveBeenLastCalledWith(true);
  await user.click(screen.getByRole('button', { name: 'Cancel download' }));
  expect(bridge.invoke).toHaveBeenCalledWith('cancel_model_install');
  await act(async () => { resolveCatalog([bundle, multimodalBundle]); });
  expect(screen.getByText('Gemma 4 E2B')).toBeTruthy();
  await act(async () => { finish(runtime); });
  await waitFor(() => expect(busy).toHaveBeenLastCalledWith(false));
  expect(screen.getByText('Gemma 4 E2B')).toBeTruthy();
  expect(screen.getByText(/Download verified/)).toBeTruthy();
});
it('keeps Gemma selected and reports cancellation after a remounted installation ends', async () => {
  let rejectInstall!: (error: Error) => void;
  let remounted = false;
  let terminal: { bundle_id: string | null; phase: string; downloaded_bytes: number; total_bytes: number; current_file: string | null; error: string | null } = idle;
  const activeGemma = { bundle_id: multimodalBundle.id, phase: 'downloading' as const, downloaded_bytes: 12, total_bytes: multimodalBundle.total_bytes, current_file: 'gemma-4-E2B-it-Q4_0.gguf', error: null };
  bridge.invoke.mockImplementation((command: string) => {
    if (command === 'model_catalog') return Promise.resolve([bundle, multimodalBundle]);
    if (command === 'model_install_status') return Promise.resolve(remounted ? terminal : idle);
    if (command === 'install_model') return new Promise((_resolve, reject) => { rejectInstall = reject; });
    if (command === 'cancel_model_install') {
      terminal = { ...activeGemma, phase: 'cancelled' };
      return Promise.resolve();
    }
    throw Error(command);
  });
  const user = userEvent.setup();
  const first = render(<ModelDownload embedding={false} disabled={false} onInstalled={vi.fn()} onBusyChange={vi.fn()} />);
  const select = await screen.findByLabelText('Answer model');
  await user.selectOptions(select, multimodalBundle.id);
  await user.click(screen.getByRole('button', { name: 'Download and select model' }));
  first.unmount();
  remounted = true;
  render(<ModelDownload embedding={false} disabled={false} onInstalled={vi.fn()} onBusyChange={vi.fn()} />);
  await screen.findByText('Gemma 4 E2B');
  await user.click(screen.getByRole('button', { name: 'Cancel download' }));
  await act(async () => { rejectInstall(Error('Cancelled')); });
  await screen.findByText('Download cancelled.');
  expect(screen.getByText('Gemma 4 E2B')).toBeTruthy();
});
it('keeps an unresolved active download cancellable when the catalog fails', async () => {
  let finish!: (value: unknown) => void;
  let remounted = false;
  const activeGemma = { bundle_id: multimodalBundle.id, phase: 'verifying' as const, downloaded_bytes: multimodalBundle.total_bytes, total_bytes: multimodalBundle.total_bytes, current_file: 'mmproj-gemma-4-E2B-it-Q8_0.gguf', error: null };
  bridge.invoke.mockImplementation((command: string) => {
    if (command === 'model_catalog') return remounted ? Promise.reject(Error('Catalog unavailable')) : Promise.resolve([bundle, multimodalBundle]);
    if (command === 'model_install_status') return Promise.resolve(remounted ? activeGemma : idle);
    if (command === 'install_model') return new Promise(resolve => { finish = resolve; });
    if (command === 'cancel_model_install') return Promise.resolve();
    throw Error(command);
  });
  const user = userEvent.setup();
  const first = render(<ModelDownload embedding={false} disabled={false} onInstalled={vi.fn()} onBusyChange={vi.fn()} />);
  const select = await screen.findByLabelText('Answer model');
  await user.selectOptions(select, multimodalBundle.id);
  await user.click(screen.getByRole('button', { name: 'Download and select model' }));
  first.unmount();
  remounted = true;
  const busy = vi.fn();
  render(<ModelDownload embedding={false} disabled={false} onInstalled={vi.fn()} onBusyChange={busy} />);
  await screen.findByText(/Verifying downloaded files/);
  await screen.findByRole('alert');
  expect(busy).toHaveBeenLastCalledWith(true);
  await user.click(screen.getByRole('button', { name: 'Cancel download' }));
  expect(bridge.invoke).toHaveBeenCalledWith('cancel_model_install');
  await act(async () => { finish(runtime); });
  await waitFor(() => expect(busy).toHaveBeenLastCalledWith(false));
});
it('blocks a second bundle globally and allows cancellation while the parent is disabled', async () => {
  let finish!: (value: unknown) => void; mock(() => new Promise(resolve => { finish = resolve; }));
  const user = userEvent.setup(); const busy = vi.fn();
  const view = render(<><ModelDownload embedding={false} disabled={false} onInstalled={vi.fn()} onBusyChange={busy} /><ModelDownload embedding disabled={false} onInstalled={vi.fn()} onBusyChange={vi.fn()} /></>);
  await screen.findByText('EmbeddingGemma 2');
  await user.click(screen.getAllByRole('button', { name: 'Download and select model' })[0]);
  expect((screen.getAllByRole('button', { name: 'Download and select model' })[1] as HTMLButtonElement).disabled).toBe(true);
  view.rerender(<><ModelDownload embedding={false} disabled onInstalled={vi.fn()} onBusyChange={busy} /><ModelDownload embedding disabled onInstalled={vi.fn()} onBusyChange={vi.fn()} /></>);
  await user.click(screen.getByRole('button', { name: 'Cancel download' }));
  expect(bridge.invoke).toHaveBeenCalledWith('cancel_model_install');
  await act(async () => { finish(runtime); });
});
it('surfaces an install failure and enables retry', async () => {
  mock(async () => { throw Error('Selection changed during download'); }); const installed = vi.fn(); const user = userEvent.setup();
  render(<ModelDownload embedding={false} disabled={false} onInstalled={installed} onBusyChange={vi.fn()} />);
  await screen.findByText('Qwen3 1.7B');
  await user.click(screen.getByRole('button', { name: 'Download and select model' }));
  await screen.findByRole('alert');
  expect(installed).not.toHaveBeenCalled();
  expect((screen.getByRole('button', { name: 'Download and select model' }) as HTMLButtonElement).disabled).toBe(false);
});
it('ignores a pending status read after installation finishes and does not call back after unmount', async () => {
  let resolveStatus!: (value: unknown) => void; let finish!: (value: unknown) => void; let reads = 0;
  mock(() => new Promise(resolve => { finish = resolve; }));
  const implementation = bridge.invoke.getMockImplementation()!;
  bridge.invoke.mockImplementation((command: string, args?: unknown) => command === 'model_install_status' && reads++ > 0 ? new Promise(resolve => { resolveStatus = resolve; }) : implementation(command, args));
  const originalTimer = globalThis.setInterval; let poll!: () => void;
  const timer = vi.spyOn(globalThis, 'setInterval').mockImplementation(((fn: () => void, delay: number) => { if (delay === 1000) poll = fn; return originalTimer(fn, delay); }) as typeof setInterval);
  try {
    const installed = vi.fn(); const user = userEvent.setup();
    const view = render(<ModelDownload embedding={false} disabled={false} onInstalled={installed} onBusyChange={vi.fn()} />);
    await screen.findByText('Qwen3 1.7B');
    await act(async () => { await Promise.resolve(); });
    await act(async () => { poll(); });
    await user.click(screen.getByRole('button', { name: 'Download and select model' }));
    await act(async () => { finish(runtime); });
    await act(async () => { resolveStatus(idle); });
    expect(screen.getByText(/Download verified/)).toBeTruthy();
    await user.click(screen.getByRole('button', { name: 'Download and select model' }));
    view.unmount();
    await act(async () => { finish(runtime); });
    expect(installed).toHaveBeenCalledTimes(1);
  } finally { timer.mockRestore(); }
});
it('shows a background verification after mounting and permits cancellation', async () => {
  mock(); const implementation = bridge.invoke.getMockImplementation()!;
  bridge.invoke.mockImplementation((command: string, args?: unknown) => command === 'model_install_status' ? Promise.resolve({ ...idle, bundle_id: bundle.id, phase: 'verifying', downloaded_bytes: bundle.total_bytes, total_bytes: bundle.total_bytes, current_file: 'model.gguf' }) : implementation(command, args));
  const busy = vi.fn(); const user = userEvent.setup();
  render(<ModelDownload embedding={false} disabled onInstalled={vi.fn()} onBusyChange={busy} />);
  await screen.findByText(/Verifying downloaded files/);
  expect(busy).toHaveBeenLastCalledWith(true);
  expect(screen.getByRole('progressbar').getAttribute('value')).toBe(String(bundle.total_bytes));
  await user.click(screen.getByRole('button', { name: 'Cancel download' }));
  expect(bridge.invoke).toHaveBeenCalledWith('cancel_model_install');
});
it('reports cancellation and permits a new attempt', async () => {
  mock(async () => { throw Error('Cancelled'); }); const implementation = bridge.invoke.getMockImplementation()!; let reads = 0;
  bridge.invoke.mockImplementation((command: string, args?: unknown) => command === 'model_install_status' && reads++ > 0 ? Promise.resolve({ ...idle, bundle_id: bundle.id, phase: 'cancelled' }) : implementation(command, args));
  const user = userEvent.setup(); const installed = vi.fn();
  render(<ModelDownload embedding={false} disabled={false} onInstalled={installed} onBusyChange={vi.fn()} />);
  await screen.findByText('Qwen3 1.7B');
  await user.click(screen.getByRole('button', { name: 'Download and select model' }));
  await screen.findByText('Download cancelled.');
  expect(installed).not.toHaveBeenCalled();
  expect(screen.queryByRole('alert')).toBeNull();
  expect((screen.getByRole('button', { name: 'Download and select model' }) as HTMLButtonElement).disabled).toBe(false);
});
it('retains a selection failure after later polls report verified files', async () => {
  mock(async () => { throw Error('Runtime configuration changed. Retry to select the downloaded files.'); });
  const implementation = bridge.invoke.getMockImplementation()!; let reads = 0;
  const verified = { ...idle, bundle_id: bundle.id, phase: 'ready', downloaded_bytes: bundle.total_bytes, total_bytes: bundle.total_bytes };
  bridge.invoke.mockImplementation((command: string, args?: unknown) => command === 'model_install_status' && reads++ > 0 ? Promise.resolve(verified) : implementation(command, args));
  const originalTimer = globalThis.setInterval; let poll!: () => void;
  const timer = vi.spyOn(globalThis, 'setInterval').mockImplementation(((fn: () => void, delay: number) => { if (delay === 1000) poll = fn; return originalTimer(fn, delay); }) as typeof setInterval);
  try {
    const installed = vi.fn(); const user = userEvent.setup();
    render(<ModelDownload embedding={false} disabled={false} onInstalled={installed} onBusyChange={vi.fn()} />);
    await screen.findByText('Qwen3 1.7B');
    await user.click(screen.getByRole('button', { name: 'Download and select model' }));
    await screen.findByRole('alert');
    await act(async () => { poll(); });
    expect(screen.getByRole('alert').textContent).toContain('Retry to select the downloaded files');
    expect(screen.getByText(/Download verified/)).toBeTruthy();
    expect(screen.queryByText(/model selected/i)).toBeNull();
    expect(installed).not.toHaveBeenCalled();
    expect(screen.getByText(/1.00 GiB download/)).toBeTruthy();
  } finally { timer.mockRestore(); }
});
