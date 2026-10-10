import { afterEach, expect, it, vi } from 'vitest';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { StrictMode } from 'react';
import userEvent from '@testing-library/user-event';
import MemoryPolicy from './MemoryPolicy';

const bridge = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: bridge.invoke }));
afterEach(() => { cleanup(); bridge.invoke.mockReset(); });

it('switches to one-model mode only after the persisted policy is loaded', async () => {
  bridge.invoke.mockImplementation(async (command: string, args?: { policy: string }) => {
    if (command === 'memory_policy') return 'both';
    if (command === 'set_memory_policy') return args?.policy;
    throw Error(command);
  });
  const changed = vi.fn(); const user = userEvent.setup();
  render(<MemoryPolicy disabled={false} onChange={changed} />);
  const control = await screen.findByLabelText('Keep one managed model loaded');
  await user.click(control);
  await waitFor(() => expect(bridge.invoke).toHaveBeenCalledWith('set_memory_policy', { policy: 'single' }));
  expect(changed).toHaveBeenLastCalledWith('single');
  expect(screen.getByText(/One managed model is kept loaded at a time/)).toBeTruthy();
});

it('retains the policy when the backend rejects a live switch', async () => {
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'memory_policy') return 'both';
    if (command === 'set_memory_policy') throw Error('Stop both managed models first.');
    throw Error(command);
  });
  const user = userEvent.setup(); render(<MemoryPolicy disabled={false} onChange={vi.fn()} />);
  await user.click(await screen.findByLabelText('Keep one managed model loaded'));
  await screen.findByRole('alert');
  expect((screen.getByLabelText('Keep one managed model loaded') as HTMLInputElement).checked).toBe(false);
});

it.each([
  ['backend failure', async () => { throw Error('Settings unavailable'); }],
  ['invalid policy', async () => 'unknown'],
])('keeps controls disabled for an unknown initial policy: %s', async (_case, response) => {
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'memory_policy') return response();
    throw Error(command);
  });
  render(<MemoryPolicy disabled={false} onChange={vi.fn()} />);
  await screen.findByRole('alert');
  expect((screen.getByLabelText('Keep one managed model loaded') as HTMLInputElement).disabled).toBe(true);
  expect(screen.getByText('Unavailable')).toBeTruthy();
});

it('does not update local state after unmount while publishing a saved policy', async () => {
  let finish: ((value: 'single') => void) | undefined;
  bridge.invoke.mockImplementation((command: string) => {
    if (command === 'memory_policy') return Promise.resolve('both');
    if (command === 'set_memory_policy') return new Promise(resolve => { finish = resolve; });
    throw Error(command);
  });
  const changed = vi.fn(); const user = userEvent.setup();
  const view = render(<MemoryPolicy disabled={false} onChange={changed} />);
  await user.click(await screen.findByLabelText('Keep one managed model loaded'));
  view.unmount();
  await act(async () => { finish!('single'); });
  expect(changed).toHaveBeenCalledTimes(2);
  expect(changed).toHaveBeenLastCalledWith('single');
});

it('ignores an abandoned StrictMode policy read that resolves after a newer save', async () => {
  let first: ((value: 'both') => void) | undefined;
  let second: ((value: 'both') => void) | undefined;
  let reads = 0;
  bridge.invoke.mockImplementation((command: string, args?: { policy: string }) => {
    if (command === 'memory_policy') return new Promise(resolve => { if (reads++ === 0) first = resolve; else second = resolve; });
    if (command === 'set_memory_policy') return Promise.resolve(args?.policy);
    throw Error(command);
  });
  const changed = vi.fn(); const user = userEvent.setup();
  render(<StrictMode><MemoryPolicy disabled={false} onChange={changed} /></StrictMode>);
  await act(async () => { second!('both'); });
  await user.click(await screen.findByLabelText('Keep one managed model loaded'));
  await waitFor(() => expect(changed).toHaveBeenLastCalledWith('single'));
  await act(async () => { first!('both'); });
  expect((screen.getByLabelText('Keep one managed model loaded') as HTMLInputElement).checked).toBe(true);
  expect(changed).toHaveBeenLastCalledWith('single');
});
