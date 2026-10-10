import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from './App';

const bridge = vi.hoisted(() => ({ invoke: vi.fn(), native: true }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: bridge.invoke, isTauri: () => bridge.native, convertFileSrc: (id: string, scheme: string) => `${scheme}://localhost/${id}` }));
const doc = { id: 'one', title: 'Launch decision.md', kind: 'md', bytes: 70, chunks: 1, created_at: '2026-10-09T00:00:00Z' };
const hit = { id: 'one:0', document_id: 'one', title: doc.title, ordinal: 0, text: 'The launch moved to Friday for security review.' };

beforeEach(() => {
  bridge.invoke.mockReset();
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'managed_runtime') return { config: { executable: null, model: null }, running: false, ready: false, managed: false, endpoint: null };
    if (command === 'media_tools') return { executable: null };
    if (command === 'workspace') return { documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local/workspace' };
    if (command === 'search_library') return [hit];
    if (command === 'read_document') return hit.text;
    if (command === 'begin_operation') return 'operation-1';
    if (command === 'cancel_operation') return false;
    throw new Error(`Unexpected command: ${command}`);
  });
});
afterEach(cleanup);

describe('desktop UI and native boundary', () => {
  it('searches actual bridge results and opens the cited original source', async () => {
    const user = userEvent.setup(); render(<App />);
    await screen.findByRole('button', { name: /^Documents\s*1$/ });
    await user.type(screen.getByLabelText('Search your library'), 'Why launch Friday?');
    await user.click(screen.getByRole('button', { name: 'Search library' }));
    await screen.findByRole('heading', { name: '1 matching passages' });
    expect(bridge.invoke).toHaveBeenCalledWith('search_library', expect.objectContaining({ query: 'Why launch Friday?', requestId: 'operation-1' }));
    await user.click(screen.getByText(doc.title).closest('button')!);
    await screen.findByRole('heading', { name: doc.title });
    expect(bridge.invoke).toHaveBeenCalledWith('read_document', { id: 'one' });
  });

  it('opens a PDF search hit on its physical page and navigates blank pages', async () => {
    const pdf = { ...doc, id: 'pdf-one', title: 'Plan.pdf', kind: 'pdf', chunks: 3 };
    const pdfHit = { ...hit, id: 'pdf-one:2', document_id: pdf.id, title: pdf.title, kind: 'pdf', page_number: 3, ordinal: 2, text: 'The decision is on page three.' };
    bridge.invoke.mockImplementation(async (command: string) => {
      if (command === 'workspace') return { documents: [pdf], endpoint: 'http://127.0.0.1:8080', data_path: '/local' };
      if (command === 'begin_operation') return 'search-pdf';
      if (command === 'search_library') return [pdfHit];
      if (command === 'read_pdf_pages') return [{ number: 1, text: 'Cover page' }, { number: 2, text: '' }, { number: 3, text: 'The decision is on page three.' }];
      throw new Error(command);
    });
    const user = userEvent.setup(); render(<App />);
    await screen.findByRole('button', { name: /^Documents\s*1$/ });
    await user.type(screen.getByLabelText('Search your library'), 'Where is the decision?');
    await user.click(screen.getByRole('button', { name: 'Search library' }));
    expect(screen.getByText('Page 3')).toBeTruthy();
    await user.click(screen.getByText(pdf.title).closest('button')!);
    await screen.findByRole('heading', { name: 'Page 3' });
    expect(bridge.invoke).toHaveBeenCalledWith('read_pdf_pages', { id: pdf.id });
    await user.click(screen.getByRole('button', { name: 'Previous page' }));
    expect(screen.getByRole('heading', { name: 'Page 2' })).toBeTruthy();
    expect(screen.getByText('No extracted text for this page.')).toBeTruthy();
  });

  it('opens a PDF from the library at page one with a page selector', async () => {
    const pdf = { ...doc, id: 'pdf-one', title: 'Plan.pdf', kind: 'pdf', chunks: 3 };
    bridge.invoke.mockImplementation(async (command: string) => {
      if (command === 'workspace') return { documents: [pdf], endpoint: 'http://127.0.0.1:8080', data_path: '/local' };
      if (command === 'read_pdf_pages') return [{ number: 1, text: 'Cover page' }, { number: 2, text: '' }, { number: 3, text: 'Decision' }];
      throw new Error(command);
    });
    const user = userEvent.setup(); render(<App />);
    await user.click(await screen.findByRole('button', { name: /^Documents\s*1$/ }));
    await user.click(screen.getByText(pdf.title).closest('button')!);
    await screen.findByRole('heading', { name: 'Page 1' });
    expect((screen.getByLabelText('PDF page') as HTMLSelectElement).value).toBe('1');
    expect((screen.getByRole('button', { name: 'Previous page' }) as HTMLButtonElement).disabled).toBe(true);
  });

  it('retains a PDF citation page when opening an answer source', async () => {
    const pdf = { ...doc, id: 'pdf-one', title: 'Plan.pdf', kind: 'pdf', chunks: 3 };
    const pdfHit = { ...hit, id: 'pdf-one:2', document_id: pdf.id, title: pdf.title, kind: 'pdf', page_number: 3, ordinal: 2, text: 'The decision is on page three.' };
    bridge.invoke.mockImplementation(async (command: string) => {
      if (command === 'workspace') return { documents: [pdf], endpoint: 'http://127.0.0.1:8080', data_path: '/local' };
      if (command === 'search_library') return [pdfHit];
      if (command === 'begin_operation') return 'answer-pdf';
      if (command === 'ask_library') return { answer: { answer: 'Page three.', source_ids: [pdfHit.id], insufficient_evidence: false }, sources: [pdfHit], elapsed_ms: 20, prompt_tokens: 10, completion_tokens: 2, cached_tokens: null };
      if (command === 'cancel_operation') return false;
      if (command === 'read_pdf_pages') return [{ number: 1, text: 'Cover page' }, { number: 2, text: '' }, { number: 3, text: 'The decision is on page three.' }];
      throw new Error(command);
    });
    const user = userEvent.setup(); render(<App />);
    await screen.findByRole('button', { name: /^Documents\s*1$/ });
    await user.click(screen.getByRole('button', { name: 'Why did we postpone the launch?' }));
    await user.click(await screen.findByRole('button', { name: 'Generate answer' }));
    await user.click(await screen.findByRole('button', { name: /1\. Plan\.pdf · Page 3/ }));
    await screen.findByRole('heading', { name: 'Page 3' });
  });

  it('keeps retrieved evidence available after generation fails', async () => {
    bridge.invoke.mockImplementation(async (command: string) => {
      if (command === 'managed_runtime') return { config: { executable: null, model: null }, running: false, ready: false, managed: false, endpoint: null };
    if (command === 'media_tools') return { executable: null };
    if (command === 'workspace') return { documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local/workspace' };
      if (command === 'search_library') return [hit];
      if (command === 'begin_operation') return 'answer-timeout';
      if (command === 'ask_library') throw new Error('Local generation timed out');
      if (command === 'cancel_operation') return false;
    });
    const user = userEvent.setup(); render(<App />);
    await screen.findByRole('button', { name: /^Documents\s*1$/ });
    await user.click(screen.getByRole('button', { name: 'Why did we postpone the launch?' }));
    await user.click(await screen.findByRole('button', { name: 'Generate answer' }));
    await screen.findByRole('alert');
    expect(screen.getByText(doc.title).closest('button')!).toBeTruthy();
  });

  it('clears the old connected model when the endpoint changes and a new check fails', async () => {
    let checks = 0;
    bridge.invoke.mockImplementation(async (command: string, args?: { value: string }) => {
      if (command === 'memory_policy') return 'both';
      if (command === 'managed_runtime') return { config: { executable: null, model: null }, running: false, ready: false, managed: false, endpoint: null };
    if (command === 'media_tools') return { executable: null };
    if (command === 'workspace') return { documents: [], endpoint: 'http://127.0.0.1:8080', data_path: '/local/workspace' };
      if (command === 'save_endpoint') return args?.value;
      if (command === 'engine_status') {
        if (checks++ === 0) return { connected: true, model: 'Old model', endpoint: 'http://127.0.0.1:8080', message: 'Connected' };
        throw new Error('Local engine returned invalid JSON');
      }
    });
    const user = userEvent.setup(); render(<App />);
    await user.click(screen.getByRole('button', { name: 'Local runtime' }));
    await user.click(screen.getByRole('button', { name: 'Save & check' }));
    await screen.findByText('Old model');
    await user.clear(screen.getByLabelText('Local server address'));
    await user.type(screen.getByLabelText('Local server address'), 'http://127.0.0.1:18080');
    expect(screen.queryByText('Old model')).toBeNull();
    await user.click(screen.getByRole('button', { name: 'Save & check' }));
    await screen.findByRole('alert');
    expect(screen.queryByText('Connected')).toBeNull();
    expect(screen.getByText('Awaiting connection')).toBeTruthy();
  });

  it('requires confirmation and refreshes the library after deletion', async () => {
    let removed = false;
    bridge.invoke.mockImplementation(async (command: string) => {
      if (command === 'managed_runtime') return { config: { executable: null, model: null }, running: false, ready: false, managed: false, endpoint: null };
    if (command === 'media_tools') return { executable: null };
    if (command === 'workspace') return { documents: removed ? [] : [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local/workspace' };
      if (command === 'delete_document') { removed = true; return; }
    });
    const user = userEvent.setup(); render(<App />);
    await user.click(await screen.findByRole('button', { name: /^Documents\s*1$/ }));
    await user.click(screen.getByRole('button', { name: 'Remove Launch decision.md' }));
    expect(bridge.invoke).not.toHaveBeenCalledWith('delete_document', expect.anything());
    await user.click(screen.getByRole('button', { name: 'Remove' }));
    await waitFor(() => expect(screen.getByRole('button', { name: /^Documents\s*0$/ })).toBeTruthy());
    expect(bridge.invoke).toHaveBeenCalledWith('delete_document', { id: 'one' });
  });
});

it('discards a delayed engine status after the managed model stops', async () => {
  let resolveEngine: ((value: unknown) => void) | undefined;
  const runtime = { config: { executable: '/engine/llama-server', model: '/models/small.gguf' }, running: false, ready: false, managed: false, endpoint: null };
  bridge.invoke.mockImplementation(async (command: string, args?: { embedding?: boolean }) => {
    if (args?.embedding) return { ...runtime, config: { executable: null, model: null } };
    if (command === 'media_tools') return { executable: null };
    if (command === 'workspace') return { documents: [], endpoint: 'http://127.0.0.1:8080', data_path: '/local' };
    if (command === 'managed_runtime' || command === 'stop_runtime') return runtime;
    if (command === 'start_runtime') return { ...runtime, running: true, ready: true, managed: true };
    if (command === 'engine_status') return new Promise(resolve => { resolveEngine = resolve; });
    throw new Error(`Unexpected command: ${command}`);
  });
  const user = userEvent.setup(); render(<App />);
  await user.click(screen.getByRole('button', { name: 'Local runtime' }));
  await screen.findByText('/models/small.gguf');
  await user.click(screen.getByRole('button', { name: 'Start model' }));
  await screen.findByText('Running');
  await user.click(screen.getByRole('button', { name: 'Stop model' }));
  await waitFor(() => expect((screen.getByRole('button', { name: 'Stop model' }) as HTMLButtonElement).disabled).toBe(true));
  await act(async () => { resolveEngine!({ connected: true, model: 'Stale model', endpoint: 'http://127.0.0.1:8080', message: 'Connected' }); });
  expect(screen.queryByText('Stale model')).toBeNull();
  expect(screen.getByText('Awaiting connection')).toBeTruthy();
});

it('uses the selected semantic retrieval mode for search and answer evidence', async () => {
  const user = userEvent.setup(); render(<App />);
  await screen.findByRole('button', { name: /^Documents\s*1$/ });
  await user.click(screen.getByLabelText(/Use semantic search/));
  await user.click(screen.getByRole('button', { name: 'Why did we postpone the launch?' }));
  expect(bridge.invoke).toHaveBeenCalledWith('search_library', expect.objectContaining({ query: 'Why did we postpone the launch?', semantic: true }));
  await user.click(await screen.findByRole('button', { name: 'Generate answer' }));
  expect(bridge.invoke).toHaveBeenCalledWith('ask_library', expect.objectContaining({ question: 'Why did we postpone the launch?', cache: true, semantic: true, requestId: 'operation-1' }));
  await screen.findByRole('alert');
});

it('opens a retrieved video at its indexed timestamp', async () => {
  const media = { ...doc, kind: 'video', title: 'Demo.mp4' };
  const mediaHit = { ...hit, kind: 'video', title: media.title, start_ms: 10000, end_ms: 20000, text: 'Video audio 00:10–00:20' };
  bridge.invoke.mockImplementation(async (command: string) => {
    if (command === 'workspace') return { documents: [media], endpoint: 'http://127.0.0.1:8080', data_path: '/local' };
    if (command === 'begin_operation') return 'search-media';
    if (command === 'search_library') return [mediaHit];
    if (command === 'read_document') return 'Stored converted media preview';
    throw new Error(command);
  });
  const user = userEvent.setup(); render(<App />);
  await screen.findByRole('button', { name: /^Documents\s*1$/ });
  await user.click(screen.getByLabelText(/Use semantic search/));
  await user.click(screen.getByRole('button', { name: 'What is our infrastructure budget?' }));
  await user.click((await screen.findByText('Demo.mp4')).closest('button')!);
  await screen.findByLabelText('Demo.mp4 media preview');
  expect(screen.getByText('Matched interval: 0:10–0:20')).toBeTruthy();
});

describe('answer cancellation', () => {
  async function retrieveEvidence(user: ReturnType<typeof userEvent.setup>) {
    render(<App />);
    await screen.findByRole('button', { name: /^Documents\s*1$/ });
    await user.click(screen.getByRole('button', { name: 'Why did we postpone the launch?' }));
    await user.click(await screen.findByRole('button', { name: 'Generate answer' }));
  }

  it('cancels the matching request and retains hits for a retry', async () => {
    let rejectAnswer: ((reason?: unknown) => void) | undefined;
    let begins = 0;
    bridge.invoke.mockImplementation((command: string) => {
      if (command === 'workspace') return Promise.resolve({ documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' });
      if (command === 'search_library') return Promise.resolve([hit]);
      if (command === 'begin_operation') return Promise.resolve(++begins === 1 ? 'search-1' : `answer-${begins - 1}`);
      if (command === 'ask_library') return begins === 2 ? new Promise((_resolve, reject) => { rejectAnswer = reject; }) : Promise.resolve({ answer: { answer: 'Friday.', source_ids: [hit.id], insufficient_evidence: false }, sources: [hit], elapsed_ms: 1, prompt_tokens: 1, completion_tokens: 1, cached_tokens: null });
      if (command === 'cancel_operation') return Promise.resolve(true);
      throw new Error(command);
    });
    const user = userEvent.setup();
    await retrieveEvidence(user);
    await user.click(screen.getByRole('button', { name: 'Cancel answer' }));
    expect(bridge.invoke).toHaveBeenCalledWith('cancel_operation', { requestId: 'answer-1' });
    await act(async () => { rejectAnswer!(new Error('Answer cancelled.')); });
    expect(screen.getByText(doc.title).closest('button')).toBeTruthy();
    const retry = await screen.findByRole('button', { name: 'Generate answer' });
    expect((retry as HTMLButtonElement).disabled).toBe(false);
    await user.click(retry);
    await screen.findByText('Friday.');
    expect(bridge.invoke).toHaveBeenCalledWith('ask_library', expect.objectContaining({ requestId: 'answer-2' }));
  });

  it('ignores a late success after cancellation', async () => {
    let resolveAnswer: ((value: unknown) => void) | undefined;
    bridge.invoke.mockImplementation((command: string) => {
      if (command === 'workspace') return Promise.resolve({ documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' });
      if (command === 'search_library') return Promise.resolve([hit]);
      if (command === 'begin_operation') return Promise.resolve('answer-late');
      if (command === 'ask_library') return new Promise(resolve => { resolveAnswer = resolve; });
      if (command === 'cancel_operation') return Promise.resolve(true);
      throw new Error(command);
    });
    const user = userEvent.setup();
    await retrieveEvidence(user);
    await user.click(screen.getByRole('button', { name: 'Cancel answer' }));
    await act(async () => { resolveAnswer!({ answer: { answer: 'Stale answer.', source_ids: [hit.id], insufficient_evidence: false }, sources: [hit], elapsed_ms: 1, prompt_tokens: 1, completion_tokens: 1, cached_tokens: null }); });
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Cancel answer' })).toBeNull());
    expect(screen.queryByText('Stale answer.')).toBeNull();
  });

  it('clears busy state when beginning an answer fails', async () => {
    bridge.invoke.mockImplementation(async (command: string) => {
      if (command === 'workspace') return { documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' };
      if (command === 'search_library') return [hit];
      if (command === 'begin_operation') return bridge.invoke.mock.calls.filter(([name]) => name === 'begin_operation').length === 1 ? 'search-failure' : Promise.reject(new Error('Reservation unavailable'));
      throw new Error(command);
    });
    const user = userEvent.setup();
    await retrieveEvidence(user);
    await screen.findByRole('alert');
    expect((screen.getByRole('button', { name: 'Generate answer' }) as HTMLButtonElement).disabled).toBe(false);
  });

  it('shows cancellation failure and allows retrying cancellation', async () => {
    let attempts = 0;
    bridge.invoke.mockImplementation((command: string) => {
      if (command === 'workspace') return Promise.resolve({ documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' });
      if (command === 'search_library') return Promise.resolve([hit]);
      if (command === 'begin_operation') return Promise.resolve('answer-retry');
      if (command === 'ask_library') return new Promise(() => {});
      if (command === 'cancel_operation') return ++attempts === 1 ? Promise.reject(new Error('Bridge unavailable')) : Promise.resolve(true);
      throw new Error(command);
    });
    const user = userEvent.setup();
    await retrieveEvidence(user);
    await user.click(screen.getByRole('button', { name: 'Cancel answer' }));
    await screen.findByText(/Unable to cancel answer/);
    await user.click(screen.getByRole('button', { name: 'Cancel answer' }));
    await waitFor(() => expect(attempts).toBe(2));
  });

  it('cancels a reservation that arrives after the user cancels and never asks', async () => {
    let resolveBegin: ((value: string) => void) | undefined;
    let begins = 0;
    bridge.invoke.mockImplementation((command: string) => {
      if (command === 'workspace') return Promise.resolve({ documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' });
      if (command === 'search_library') return Promise.resolve([hit]);
      if (command === 'begin_operation') return ++begins === 1 ? Promise.resolve('search-pending') : new Promise<string>(resolve => { resolveBegin = resolve; });
      if (command === 'cancel_operation') return Promise.resolve(true);
      if (command === 'ask_library') throw new Error('ask_library must not run after cancellation');
      throw new Error(command);
    });
    const user = userEvent.setup();
    await retrieveEvidence(user);
    await user.click(screen.getByRole('button', { name: 'Cancel answer' }));
    await act(async () => { resolveBegin!('answer-pending'); });
    await waitFor(() => expect(bridge.invoke).toHaveBeenCalledWith('cancel_operation', { requestId: 'answer-pending' }));
    expect(bridge.invoke).not.toHaveBeenCalledWith('ask_library', expect.anything());
    await screen.findByText('Answer generation cancelled.');
    expect(screen.getByText(doc.title).closest('button')).toBeTruthy();
  });

  it('cancels once when unmounted before begin_operation resolves', async () => {
    let resolveBegin: ((value: string) => void) | undefined;
    let begins = 0;
    bridge.invoke.mockImplementation((command: string) => {
      if (command === 'workspace') return Promise.resolve({ documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' });
      if (command === 'search_library') return Promise.resolve([hit]);
      if (command === 'begin_operation') return ++begins === 1 ? Promise.resolve('search-unmount') : new Promise<string>(resolve => { resolveBegin = resolve; });
      if (command === 'cancel_operation') return Promise.resolve(true);
      if (command === 'ask_library') throw new Error('ask_library must not run after unmount');
      throw new Error(command);
    });
    const user = userEvent.setup();
    const rendered = render(<App />);
    await screen.findByRole('button', { name: /^Documents\s*1$/ });
    await user.click(screen.getByRole('button', { name: 'Why did we postpone the launch?' }));
    await user.click(await screen.findByRole('button', { name: 'Generate answer' }));
    rendered.unmount();
    await act(async () => { resolveBegin!('answer-unmounted'); });
    await waitFor(() => expect(bridge.invoke).toHaveBeenCalledWith('cancel_operation', { requestId: 'answer-unmounted' }));
    expect(bridge.invoke.mock.calls.filter(([command]) => command === 'cancel_operation')).toHaveLength(1);
    expect(bridge.invoke).not.toHaveBeenCalledWith('ask_library', expect.anything());
  });

  it('does not ask after cancellation fails before begin_operation resolves', async () => {
    let resolveBegin: ((value: string) => void) | undefined;
    let begins = 0;
    bridge.invoke.mockImplementation((command: string) => {
      if (command === 'workspace') return Promise.resolve({ documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' });
      if (command === 'search_library') return Promise.resolve([hit]);
      if (command === 'begin_operation') return ++begins === 1 ? Promise.resolve('search-pending-error') : new Promise<string>(resolve => { resolveBegin = resolve; });
      if (command === 'cancel_operation') return Promise.reject(new Error('Bridge unavailable'));
      if (command === 'ask_library') throw new Error('ask_library must not run after cancellation');
      throw new Error(command);
    });
    const user = userEvent.setup();
    await retrieveEvidence(user);
    await user.click(screen.getByRole('button', { name: 'Cancel answer' }));
    await act(async () => { resolveBegin!('answer-pending-error'); });
    await screen.findByText(/Unable to cancel answer: Error: Bridge unavailable/);
    expect(bridge.invoke).not.toHaveBeenCalledWith('ask_library', expect.anything());
    expect((screen.getByRole('button', { name: 'Generate answer' }) as HTMLButtonElement).disabled).toBe(false);
  });

  it('keeps cancellation available after navigating to Documents', async () => {
    bridge.invoke.mockImplementation((command: string) => {
      if (command === 'workspace') return Promise.resolve({ documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' });
      if (command === 'search_library') return Promise.resolve([hit]);
      if (command === 'begin_operation') return Promise.resolve('answer-navigation');
      if (command === 'ask_library') return new Promise(() => {});
      if (command === 'cancel_operation') return Promise.resolve(true);
      throw new Error(command);
    });
    const user = userEvent.setup();
    await retrieveEvidence(user);
    await user.click(screen.getByRole('button', { name: /^Documents\s*1$/ }));
    await user.click(screen.getByRole('button', { name: 'Cancel answer' }));
    expect(bridge.invoke).toHaveBeenCalledWith('cancel_operation', { requestId: 'answer-navigation' });
  });

  it('renders a completed answer when a delayed cancellation is declined', async () => {
    let resolveAnswer: ((value: unknown) => void) | undefined;
    let resolveCancel: ((value: boolean) => void) | undefined;
    bridge.invoke.mockImplementation((command: string) => {
      if (command === 'workspace') return Promise.resolve({ documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' });
      if (command === 'search_library') return Promise.resolve([hit]);
      if (command === 'begin_operation') return Promise.resolve('answer-declined');
      if (command === 'ask_library') return new Promise(resolve => { resolveAnswer = resolve; });
      if (command === 'cancel_operation') return new Promise<boolean>(resolve => { resolveCancel = resolve; });
      throw new Error(command);
    });
    const user = userEvent.setup();
    await retrieveEvidence(user);
    await user.click(screen.getByRole('button', { name: 'Cancel answer' }));
    await act(async () => { resolveAnswer!({ answer: { answer: 'Completed answer.', source_ids: [hit.id], insufficient_evidence: false }, sources: [hit], elapsed_ms: 1, prompt_tokens: 1, completion_tokens: 1, cached_tokens: null }); });
    expect(screen.queryByText('Completed answer.')).toBeNull();
    await act(async () => { resolveCancel!(false); });
    await screen.findByText('Completed answer.');
    await screen.findByText(/could not be cancelled because it already completed/);
  });

  it('preserves a completed answer when cancellation rejects', async () => {
    let resolveAnswer: ((value: unknown) => void) | undefined;
    let rejectCancel: ((reason?: unknown) => void) | undefined;
    bridge.invoke.mockImplementation((command: string) => {
      if (command === 'workspace') return Promise.resolve({ documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' });
      if (command === 'search_library') return Promise.resolve([hit]);
      if (command === 'begin_operation') return Promise.resolve('answer-cancel-error');
      if (command === 'ask_library') return new Promise(resolve => { resolveAnswer = resolve; });
      if (command === 'cancel_operation') return new Promise((_resolve, reject) => { rejectCancel = reject; });
      throw new Error(command);
    });
    const user = userEvent.setup();
    await retrieveEvidence(user);
    await user.click(screen.getByRole('button', { name: 'Cancel answer' }));
    await act(async () => { resolveAnswer!({ answer: { answer: 'Saved answer.', source_ids: [hit.id], insufficient_evidence: false }, sources: [hit], elapsed_ms: 1, prompt_tokens: 1, completion_tokens: 1, cached_tokens: null }); });
    await act(async () => { rejectCancel!(new Error('Bridge unavailable')); });
    await screen.findByText('Saved answer.');
    await screen.findByText(/Unable to cancel answer: Error: Bridge unavailable/);
  });

  it('shows cancellation success when acknowledgement follows an answer rejection', async () => {
    let rejectAnswer: ((reason?: unknown) => void) | undefined;
    let resolveCancel: ((value: boolean) => void) | undefined;
    bridge.invoke.mockImplementation((command: string) => {
      if (command === 'workspace') return Promise.resolve({ documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' });
      if (command === 'search_library') return Promise.resolve([hit]);
      if (command === 'begin_operation') return Promise.resolve('answer-accepted-late');
      if (command === 'ask_library') return new Promise((_resolve, reject) => { rejectAnswer = reject; });
      if (command === 'cancel_operation') return new Promise<boolean>(resolve => { resolveCancel = resolve; });
      throw new Error(command);
    });
    const user = userEvent.setup();
    await retrieveEvidence(user);
    await user.click(screen.getByRole('button', { name: 'Cancel answer' }));
    await act(async () => { rejectAnswer!(new Error('Answer cancelled.')); });
    await act(async () => { resolveCancel!(true); });
    await screen.findByText('Answer generation cancelled.');
    expect(screen.queryByRole('alert')).toBeNull();
  });
});

describe('search cancellation', () => {
  it('cancels a pending search registration without submitting search_library', async () => {
    let resolveBegin: ((value: string) => void) | undefined;
    bridge.invoke.mockImplementation((command: string) => {
      if (command === 'workspace') return Promise.resolve({ documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' });
      if (command === 'begin_operation') return new Promise<string>(resolve => { resolveBegin = resolve; });
      if (command === 'cancel_operation') return Promise.reject(new Error('Bridge unavailable'));
      if (command === 'search_library') throw new Error('search_library must not run after cancellation');
      throw new Error(command);
    });
    const user = userEvent.setup(); render(<App />);
    await screen.findByRole('button', { name: /^Documents\s*1$/ });
    await user.type(screen.getByLabelText('Search your library'), 'pending');
    await user.click(screen.getByRole('button', { name: 'Search library' }));
    await user.click(screen.getByRole('button', { name: 'Cancel search' }));
    await act(async () => { resolveBegin!('search-pending'); });
    await screen.findByText(/Unable to cancel search/);
    expect(bridge.invoke).not.toHaveBeenCalledWith('search_library', expect.anything());
  });

  it('ignores late cancelled search output and restores previous results and query', async () => {
    let searches = 0;
    let resolveSearch: ((value: unknown) => void) | undefined;
    bridge.invoke.mockImplementation((command: string) => {
      if (command === 'workspace') return Promise.resolve({ documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' });
      if (command === 'begin_operation') return Promise.resolve(`search-${++searches}`);
      if (command === 'search_library') return searches === 1 ? Promise.resolve([hit]) : new Promise(resolve => { resolveSearch = resolve; });
      if (command === 'cancel_operation') return Promise.resolve(true);
      throw new Error(command);
    });
    const user = userEvent.setup(); render(<App />);
    await screen.findByRole('button', { name: /^Documents\s*1$/ });
    const input = screen.getByLabelText('Search your library');
    await user.type(input, 'old query'); await user.click(screen.getByRole('button', { name: 'Search library' }));
    await screen.findByRole('heading', { name: '1 matching passages' });
    await user.clear(input); await user.type(input, 'new query'); await user.click(screen.getByRole('button', { name: 'Search library' }));
    await user.click(screen.getByRole('button', { name: 'Cancel search' }));
    await act(async () => { resolveSearch!([{ ...hit, text: 'stale result' }]); });
    await screen.findByText('Search cancelled.');
    expect((input as HTMLTextAreaElement).value).toBe('old query');
    expect(screen.queryByText('stale result')).toBeNull();
    expect(screen.getByText(doc.title).closest('button')).toBeTruthy();
  });

  it('publishes a completed search after a delayed rejected cancellation', async () => {
    let resolveSearch: ((value: unknown) => void) | undefined;
    let resolveCancel: ((value: boolean) => void) | undefined;
    bridge.invoke.mockImplementation((command: string) => {
      if (command === 'workspace') return Promise.resolve({ documents: [doc], endpoint: 'http://127.0.0.1:8080', data_path: '/local' });
      if (command === 'begin_operation') return Promise.resolve('search-declined');
      if (command === 'search_library') return new Promise(resolve => { resolveSearch = resolve; });
      if (command === 'cancel_operation') return new Promise<boolean>(resolve => { resolveCancel = resolve; });
      throw new Error(command);
    });
    const user = userEvent.setup(); render(<App />);
    await screen.findByRole('button', { name: /^Documents\s*1$/ });
    await user.type(screen.getByLabelText('Search your library'), 'completed');
    await user.click(screen.getByRole('button', { name: 'Search library' }));
    await user.click(screen.getByRole('button', { name: 'Cancel search' }));
    await act(async () => { resolveSearch!([hit]); });
    expect(screen.queryByRole('heading', { name: '1 matching passages' })).toBeNull();
    await act(async () => { resolveCancel!(false); });
    await screen.findByRole('heading', { name: '1 matching passages' });
    await screen.findByText(/could not be cancelled because it already completed/);
  });
});
