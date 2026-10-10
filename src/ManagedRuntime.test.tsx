import { afterEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import ManagedRuntime from './ManagedRuntime';
const bridge = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: bridge.invoke }));
afterEach(() => { cleanup(); bridge.invoke.mockReset(); });
const configured = { config: { executable: '/engine/llama-server', model: '/models/small.gguf' }, running: false, ready: false, managed: false, endpoint: null };

describe('managed runtime controls', () => {
  it('starts selected files, reports readiness, and stops the owned process', async () => {
    let status = configured;
    bridge.invoke.mockImplementation(async (command: string) => {
      if (command === 'start_runtime') status = { ...configured, running: true, ready: true, managed: true };
      if (command === 'stop_runtime') status = { ...configured, managed: true };
      return status;
    });
    const notify = vi.fn(); const user = userEvent.setup();
    render(<ManagedRuntime disabled={false} onConnectionChange={notify} />);
    await screen.findByText('/models/small.gguf');
    await user.click(screen.getByRole('button', { name: 'Start model' }));
    await screen.findByText('Running');
    expect(notify).toHaveBeenCalledWith(true);
    expect((screen.getByRole('button', { name: 'Choose model' }) as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole('button', { name: 'Stop model' }));
    await screen.findByText('Stopped');
    expect(notify).toHaveBeenLastCalledWith(false);
    expect(bridge.invoke).toHaveBeenCalledWith('stop_runtime', undefined);
  });

  it('shows startup failure and allows retry without claiming a connection', async () => {
    bridge.invoke.mockImplementation(async (command: string) => {
      if (command === 'start_runtime') throw new Error('Model failed to load');
      return configured;
    });
    const notify = vi.fn(); const user = userEvent.setup();
    render(<ManagedRuntime disabled={false} onConnectionChange={notify} />);
    await screen.findByText('/models/small.gguf');
    await user.click(screen.getByRole('button', { name: 'Start model' }));
    await screen.findByRole('alert');
    await waitFor(() => expect((screen.getByRole('button', { name: 'Start model' }) as HTMLButtonElement).disabled).toBe(false));
    expect(notify).not.toHaveBeenCalledWith(true);
    expect(screen.getByText('Stopped')).toBeTruthy();
  });
});

it('shows an armed suspended model and lets the user disarm it', async () => {
  const suspended = { ...configured, managed: true, auto_resume: true };
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'stop_runtime') return { ...configured, managed: true, auto_resume: false };
    return suspended;
  });
  const user = userEvent.setup(); render(<ManagedRuntime disabled={false} singleResident onConnectionChange={vi.fn()} />);
  await screen.findByText('On demand');
  const stop = screen.getByRole('button', { name: 'Stop model' });
  expect((stop as HTMLButtonElement).disabled).toBe(false);
  await user.click(stop);
  expect(bridge.invoke).toHaveBeenCalledWith('stop_runtime', undefined);
  await screen.findByText('Stopped');
});

it('refreshes status when the residency policy changes', async () => {
  bridge.invoke.mockImplementation(async () => configured);
  const view = render(<ManagedRuntime disabled={false} onConnectionChange={vi.fn()} />);
  await screen.findByText('/models/small.gguf');
  const before = bridge.invoke.mock.calls.filter(([command]) => command === 'managed_runtime').length;
  view.rerender(<ManagedRuntime disabled={false} singleResident onConnectionChange={vi.fn()} />);
  await waitFor(() => expect(bridge.invoke.mock.calls.filter(([command]) => command === 'managed_runtime').length).toBeGreaterThan(before));
});

it('ignores a poll response captured before a lifecycle change', async () => {
  let poll: (() => void) | undefined;
  const originalTimer = globalThis.setInterval;
  const timer = vi.spyOn(globalThis, 'setInterval').mockImplementation(((fn: () => void, delay: number) => { if (delay === 3000) poll = fn; return originalTimer(fn, delay); }) as typeof setInterval);
  let resolvePoll: ((value: typeof configured) => void) | undefined;
  let reads = 0;
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'managed_runtime' && reads++ > 0) return new Promise(resolve => { resolvePoll = resolve; });
    if (command === 'start_runtime') return { ...configured, running: true, ready: true, managed: true };
    return configured;
  });
  try {
    const user = userEvent.setup(); render(<ManagedRuntime disabled={false} onConnectionChange={vi.fn()} />);
    await screen.findByText('/models/small.gguf');
    poll!();
    await user.click(screen.getByRole('button', { name: 'Start model' }));
    await screen.findByText('Running');
    await act(async () => { resolvePoll!(configured); });
    await waitFor(() => expect(screen.getByText('Running')).toBeTruthy());
    expect((screen.getByRole('button', { name: 'Start model' }) as HTMLButtonElement).disabled).toBe(true);
  } finally { timer.mockRestore(); }
});

it('requires a projector for the embedding engine and scopes its native commands', async () => {
  const notify = vi.fn(); const user = userEvent.setup();
  bridge.invoke.mockImplementation(async (command: string) => ({ ...configured, config: { ...configured.config, projector: command === 'choose_runtime_file' ? '/models/projector.gguf' : null } }));
  render(<ManagedRuntime embedding disabled={false} onConnectionChange={notify} />);
  await screen.findByText('/models/small.gguf');
  expect((screen.getByRole('button', { name: 'Start embedding model' }) as HTMLButtonElement).disabled).toBe(true);
  await user.click(screen.getByRole('button', { name: 'Choose projector' }));
  expect(bridge.invoke).toHaveBeenCalledWith('choose_runtime_file', { kind: 'projector', embedding: true });
  await user.click(screen.getByRole('button', { name: 'Start embedding model' }));
  expect(bridge.invoke).toHaveBeenCalledWith('start_runtime', { embedding: true });
});

it('shows a selected generation projector as read-only media-answer configuration', async () => {
  bridge.invoke.mockResolvedValue({ ...configured, config: { ...configured.config, projector: '/models/gemma-projector.gguf' } });
  render(<ManagedRuntime disabled={false} onConnectionChange={vi.fn()} />);
  await screen.findByText('/models/gemma-projector.gguf');
  expect(screen.getByText('Media answers require this matching vision/audio projector.')).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Choose projector' })).toBeNull();
});

it('selects the included engine for the embedding target and keeps the selected model', async () => {
  const bundled = '/app/Contents/MacOS/llama-server';
  const selected = { ...configured, config: { ...configured.config, executable: bundled, projector: '/models/projector.gguf' } };
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'bundled_engine') return bundled;
    if (command === 'use_bundled_engine') return selected;
    return { ...configured, config: { ...configured.config, projector: '/models/projector.gguf' } };
  });
  const user = userEvent.setup();
  render(<ManagedRuntime embedding disabled={false} onConnectionChange={vi.fn()} />);
  await screen.findByRole('button', { name: 'Use included engine' });
  await user.click(screen.getByRole('button', { name: 'Use included engine' }));
  expect(bridge.invoke).toHaveBeenCalledWith('use_bundled_engine', { embedding: true });
  await screen.findByText(bundled);
  expect(screen.getByText('/models/small.gguf')).toBeTruthy();
});

it('shows no included-engine option when unavailable and preserves the runtime after a selection error', async () => {
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'bundled_engine') return '/app/Contents/MacOS/llama-server';
    if (command === 'use_bundled_engine') throw Error('Included engine is unavailable');
    return configured;
  });
  const user = userEvent.setup();
  const view = render(<ManagedRuntime disabled={false} onConnectionChange={vi.fn()} />);
  await screen.findByRole('button', { name: 'Use included engine' });
  await user.click(screen.getByRole('button', { name: 'Use included engine' }));
  await screen.findByRole('alert');
  expect(screen.getByText('/engine/llama-server')).toBeTruthy();
  view.unmount();

  bridge.invoke.mockImplementation(async (command: string) => command === 'bundled_engine' ? null : configured);
  render(<ManagedRuntime disabled={false} onConnectionChange={vi.fn()} />);
  await screen.findByText('/models/small.gguf');
  await waitFor(() => expect(screen.queryByRole('button', { name: 'Use included engine' })).toBeNull());
});

it('disables the included-engine control while the parent, runtime, or installer is busy', async () => {
  const bundled = '/app/Contents/MacOS/llama-server';
  const idle = { phase: 'idle', bundle_id: null, downloaded_bytes: 0, total_bytes: 0, current_file: null, error: null };
  let install!: (value: typeof configured) => void;
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'bundled_engine') return bundled;
    if (command === 'model_catalog') return [{ id: 'qwen3-1.7b-q4', title: 'Qwen3', description: 'Local answers', license: 'Apache-2.0', publisher: 'Qwen', total_bytes: 1073741824, source_url: 'https://huggingface.co/Qwen', embedding: false }];
    if (command === 'model_install_status') return idle;
    if (command === 'install_model') return new Promise(resolve => { install = resolve; });
    return configured;
  });
  const user = userEvent.setup();
  const view = render(<ManagedRuntime disabled onConnectionChange={vi.fn()} />);
  const button = await screen.findByRole('button', { name: 'Use included engine' });
  expect((button as HTMLButtonElement).disabled).toBe(true);
  view.rerender(<ManagedRuntime disabled={false} onConnectionChange={vi.fn()} />);
  await waitFor(() => expect((screen.getByRole('button', { name: 'Use included engine' }) as HTMLButtonElement).disabled).toBe(false));
  await user.click(screen.getByRole('button', { name: 'Download and select model' }));
  expect((screen.getByRole('button', { name: 'Use included engine' }) as HTMLButtonElement).disabled).toBe(true);
  await act(async () => { install(configured); });
});

it('disables the included-engine control while a runtime is loading', async () => {
  const bundled = '/app/Contents/MacOS/llama-server';
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'bundled_engine') return bundled;
    if (command === 'model_catalog') return [];
    if (command === 'model_install_status') return { phase: 'idle', bundle_id: null, downloaded_bytes: 0, total_bytes: 0, current_file: null, error: null };
    return { ...configured, running: true, ready: false, managed: true };
  });
  render(<ManagedRuntime disabled={false} onConnectionChange={vi.fn()} />);
  const button = await screen.findByRole('button', { name: 'Use included engine' });
  expect((button as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText('Loading')).toBeTruthy();
});

it('reports an included-engine availability failure while retaining manual setup', async () => {
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'bundled_engine') throw Error('Resource lookup failed');
    return configured;
  });
  render(<ManagedRuntime disabled={false} onConnectionChange={vi.fn()} />);
  await screen.findByText('/models/small.gguf');
  expect((await screen.findByRole('alert')).textContent).toContain('Could not check for the included engine');
  expect(screen.getByRole('button', { name: 'Choose engine' })).toBeTruthy();
});

it('selects a downloaded model while preserving the engine and ignores an older runtime poll', async () => {
  const selected = { ...configured, config: { ...configured.config, model: '/download/qwen.gguf' } };
  const idle = { phase: 'idle', bundle_id: null, downloaded_bytes: 0, total_bytes: 0, current_file: null, error: null };
  let resolveOld!: (value: typeof configured) => void;
  let finish!: (value: typeof configured) => void;
  let reads = 0; let poll!: () => void; let latest = configured;
  const originalTimer = globalThis.setInterval;
  const timer = vi.spyOn(globalThis, 'setInterval').mockImplementation(((fn: () => void, delay: number) => { if (delay === 3000) poll = fn; return originalTimer(fn, delay); }) as typeof setInterval);
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'model_catalog') return [{ id: 'qwen3-1.7b-q4', title: 'Qwen3', description: 'Local answers', license: 'Apache-2.0', publisher: 'Qwen', total_bytes: 1073741824, source_url: 'https://huggingface.co/Qwen', embedding: false }];
    if (command === 'model_install_status') return idle;
    if (command === 'install_model') return new Promise(resolve => { finish = resolve; });
    if (command === 'managed_runtime' && reads++ === 1) return new Promise(resolve => { resolveOld = resolve; });
    return latest;
  });
  try {
    const user = userEvent.setup(); render(<ManagedRuntime disabled={false} onConnectionChange={vi.fn()} />);
    await screen.findByText('/models/small.gguf');
    await act(async () => { poll(); });
    await user.click(screen.getByRole('button', { name: 'Download and select model' }));
    expect((screen.getByRole('button', { name: 'Choose engine' }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole('button', { name: 'Start model' }) as HTMLButtonElement).disabled).toBe(true);
    latest = selected;
    await act(async () => { finish(selected); });
    await screen.findByText('/download/qwen.gguf');
    await act(async () => { resolveOld(configured); });
    expect(screen.getByText('/download/qwen.gguf')).toBeTruthy();
    expect(screen.getByText('/engine/llama-server')).toBeTruthy();
    expect(bridge.invoke.mock.calls.some(([command]) => command === 'start_runtime')).toBe(false);
  } finally { timer.mockRestore(); }
});
