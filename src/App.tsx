import { useEffect, useRef, useState } from 'react';
import ManagedRuntime from './ManagedRuntime';
import EmbeddingSetup from './EmbeddingSetup';
import MemoryPolicyControl, { type MemoryPolicy } from './MemoryPolicy';
import MediaSetup from './MediaSetup';
import MediaPreview, { mediaTime } from './MediaPreview';
import PdfPreview from './PdfPreview';
import { cancelOperation, type Operation } from './operation';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { ArrowUp, BookOpen, Check, ChevronRight, CircleHelp, Cpu, FileText, FolderOpen, HardDrive, Import, Layers, LoaderCircle, Plus, Search, Settings2, ShieldCheck, Trash2, X } from 'lucide-react';

type Document = { id: string; title: string; kind: string; bytes: number; chunks: number; created_at: string };
type Hit = { id: string; document_id: string; title: string; ordinal: number; text: string; kind?: string; page_number?: number | null; start_ms?: number | null; end_ms?: number | null };
type Workspace = { documents: Document[]; endpoint: string; data_path: string };
type Engine = { connected: boolean; endpoint: string; model: string | null; message: string };
type Answer = { answer: { answer: string; source_ids: string[]; insufficient_evidence: boolean }; sources: Hit[]; elapsed_ms: number; prompt_tokens: number | null; completion_tokens: number | null; cached_tokens: number | null };
type AnswerOperation = Operation;
type View = 'ask' | 'library' | 'runtime';
const native = isTauri();
const isMedia = (kind?: string) => ['image', 'audio', 'video'].includes(kind ?? '');
const isPdf = (kind?: string) => kind === 'pdf';
const starterQuestions = ['Why did we postpone the launch?', 'What is our infrastructure budget?', 'How does the workspace protect documents?'];
const displayBytes = (n: number) => n < 1024 ? `${n} B` : n < 1024 * 1024 ? `${(n / 1024).toFixed(1)} KB` : `${(n / 1024 / 1024).toFixed(1)} MB`;

export default function App() {
  const [view, setView] = useState<View>('ask');
  const [workspace, setWorkspace] = useState<Workspace>({ documents: [], endpoint: 'http://127.0.0.1:8080', data_path: '' });
  const engineRevision = useRef(0);
  const [engine, setEngine] = useState<Engine | null>(null);
  const [endpoint, setEndpoint] = useState('http://127.0.0.1:8080');
  const [query, setQuery] = useState('');
  const [searched, setSearched] = useState('');
  const [hits, setHits] = useState<Hit[]>([]);
  const [answer, setAnswer] = useState<Answer | null>(null);
  const [answerOperation, setAnswerOperation] = useState<{ requestId?: string; cancelling: boolean } | null>(null);
  const [searchOperation, setSearchOperation] = useState<{ requestId?: string; cancelling: boolean } | null>(null);
  const [source, setSource] = useState<{ id: string; title: string; text?: string; kind?: string; pageNumber?: number | null; startMs?: number | null; endMs?: number | null } | null>(null);
  const [busy, setBusy] = useState('');
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [cache, setCache] = useState(true);
  const [semantic, setSemantic] = useState(false);
  const [memoryPolicy, setMemoryPolicy] = useState<MemoryPolicy>('both');
  const [searchedSemantic, setSearchedSemantic] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const activeAnswer = useRef<AnswerOperation | null>(null);
  const activeSearch = useRef<Operation | null>(null);
  const mounted = useRef(true);

  async function refresh() {
    const result = await invoke<Workspace>('workspace');
    setWorkspace(result); setEndpoint(result.endpoint);
  }
  useEffect(() => {
    if (native) { refresh().catch(e => setError(String(e))); }
  }, []);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      const answerOperation = activeAnswer.current;
      const searchOperation = activeSearch.current;
      if (answerOperation) void cancelAnswer(answerOperation).catch(() => undefined);
      if (searchOperation) void cancelSearch(searchOperation).catch(() => undefined);
    };
  }, []);

  async function action(label: string, fn: () => Promise<void>) {
    if (busy) return;
    setBusy(label); setError(''); setNotice('');
    try { await fn(); } catch (e) { setError(String(e)); } finally { setBusy(''); }
  }
  async function importFiles() {
    await action('Importing', async () => {
      const result = await invoke<{ imported: Document[]; errors: string[] }>('import_documents');
      await refresh(); setHits([]); setSearched(''); setAnswer(null);
      if (result.errors.length) setError(result.errors.join('\n'));
      if (result.imported.length) { setNotice(`${result.imported.length} file${result.imported.length === 1 ? '' : 's'} processed. Existing content is reused.`); setView('library'); }
    });
  }
  async function addSamples() {
    await action('Adding sample workspace', async () => {
      await invoke('add_sample_documents'); await refresh(); setView('ask'); setNotice('Three synthetic documents are ready. Try a suggested question.');
    });
  }
  function isActiveSearch(operation: Operation) {
    return activeSearch.current === operation;
  }
  async function cancelSearch(operation: Operation): Promise<boolean | undefined> {
    if (mounted.current && isActiveSearch(operation)) {
      setSearchOperation({ requestId: operation.requestId, cancelling: true });
      setBusy('Cancelling search');
    }
    return cancelOperation(operation, invoke, () => {
      if (mounted.current && isActiveSearch(operation)) {
        setSearchOperation({ requestId: operation.requestId, cancelling: false });
        setBusy('Searching');
        setError('The search could not be cancelled because it already completed.');
      }
    }, error => {
      if (mounted.current && isActiveSearch(operation)) {
        setSearchOperation({ requestId: operation.requestId, cancelling: false });
        setBusy('Searching');
        setError(`Unable to cancel search: ${String(error)}`);
      }
    });
  }
  async function search(question = query) {
    if (!question.trim()) return;
    if (busy) return;
    const operation: Operation = { cancelRequested: false };
    activeSearch.current = operation;
    setBusy('Searching'); setError(''); setNotice(''); setSearchOperation({ cancelling: false });
    try {
      const requestId = await invoke<string>('begin_operation');
      operation.requestId = requestId;
      if (mounted.current && isActiveSearch(operation)) setSearchOperation({ requestId, cancelling: operation.cancelRequested });
      if (operation.cancelRequested) {
        try { await cancelSearch(operation); } catch { /* Cancellation errors are displayed. */ }
        return;
      }
      let result: Hit[] | undefined;
      let searchError: unknown;
      try { result = await invoke<Hit[]>('search_library', { query: question, ...(semantic ? { semantic: true } : {}), requestId }); }
      catch (e) { searchError = e; }
      if (operation.cancelPromise) {
        try { await operation.cancelPromise; } catch { /* Cancellation errors are displayed. */ }
      }
      if (operation.cancellationAccepted) return;
      if (mounted.current && isActiveSearch(operation)) {
        if (result) {
          setQuery(question); setSearched(question); setSearchedSemantic(semantic); setHits(result); setAnswer(null); setView('ask'); setSource(null);
        } else if (searchError) setError(String(searchError));
      }
    } catch (e) {
      if (!operation.cancellationAccepted && mounted.current && isActiveSearch(operation)) setError(String(e));
    } finally {
      if (isActiveSearch(operation)) {
        activeSearch.current = null;
        if (mounted.current) {
          setBusy(''); setSearchOperation(null);
          if (operation.cancellationAccepted) { setQuery(searched); setNotice('Search cancelled.'); }
        }
      }
    }
  }
  async function openSource(id: string, title: string, kind?: string, pageNumber?: number | null, startMs?: number | null, endMs?: number | null) {
    await action('Opening source', async () => {
      if (isPdf(kind)) { setSource({ id, title, kind, pageNumber: pageNumber ?? 1, startMs, endMs }); return; }
      const text = await invoke<string>('read_document', { id }); setSource({ id, title, text, kind, startMs, endMs });
    });
  }
  function isActiveAnswer(operation: AnswerOperation) {
    return activeAnswer.current === operation;
  }
  async function cancelAnswer(operation: AnswerOperation): Promise<boolean | undefined> {
    if (mounted.current && isActiveAnswer(operation)) {
      setAnswerOperation({ requestId: operation.requestId, cancelling: true });
      setBusy('Cancelling answer');
    }
    return cancelOperation(operation, invoke, () => {
        if (mounted.current && isActiveAnswer(operation)) {
          setAnswerOperation({ requestId: operation.requestId, cancelling: false });
          setBusy('Generating locally');
          setError('The answer could not be cancelled because it already completed.');
        }
    }, error => {
      if (mounted.current && isActiveAnswer(operation)) {
        setAnswerOperation({ requestId: operation.requestId, cancelling: false });
        setBusy('Generating locally');
        setError(`Unable to cancel answer: ${String(error)}`);
      }
    });
  }
  async function generate() {
    if (busy || !searched) return;
    const operation: AnswerOperation = { cancelRequested: false };
    activeAnswer.current = operation;
    setBusy('Generating locally'); setError(''); setNotice(''); setAnswer(null); setAnswerOperation({ cancelling: false });
    try {
      const requestId = await invoke<string>('begin_operation');
      operation.requestId = requestId;
      if (mounted.current && isActiveAnswer(operation)) setAnswerOperation({ requestId, cancelling: operation.cancelRequested });
      if (operation.cancelRequested) {
        try { await cancelAnswer(operation); } catch { /* The cancellation error is already displayed. */ }
        return;
      }
      let result: Answer | undefined;
      let answerError: unknown;
      try { result = await invoke<Answer>('ask_library', { question: searched, cache, ...(searchedSemantic ? { semantic: true } : {}), requestId }); }
      catch (e) { answerError = e; }
      if (operation.cancelPromise) {
        try { await operation.cancelPromise; } catch { /* Cancellation errors are shown below. */ }
      }
      if (operation.cancellationAccepted) return;
      if (mounted.current && isActiveAnswer(operation)) {
        if (result) setAnswer(result);
        else if (answerError) setError(String(answerError));
      }
    } catch (e) {
      if (!operation.cancellationAccepted && mounted.current && isActiveAnswer(operation)) setError(String(e));
    } finally {
      if (isActiveAnswer(operation)) {
        activeAnswer.current = null;
        if (mounted.current) {
          setBusy(''); setAnswerOperation(null);
          if (operation.cancellationAccepted) setNotice('Answer generation cancelled.');
        }
      }
    }
  }
  async function remove(id: string) {
    await action('Removing document', async () => {
      await invoke('delete_document', { id }); await refresh(); setSource(null); setHits([]); setSearched(''); setAnswer(null); setDeleteId(null); setNotice('Document and searchable chunks removed.');
    });
  }
  async function checkEngine() {
    await action('Checking local model', async () => {
      const revision = ++engineRevision.current;
      setEngine(null);
      const saved = await invoke<string>('save_endpoint', { value: endpoint }); setEndpoint(saved);
      const status = await invoke<Engine>('engine_status');
      if (engineRevision.current === revision) setEngine(status);
    });
  }
  const documents = workspace.documents;
  const totalChunks = documents.reduce((s, d) => s + d.chunks, 0);
  const disabled = !!busy || !native;

  return <div className="app-shell">
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark"><Layers size={22} strokeWidth={1.8} /></span><span>cachella<span className="brand-dot">.</span></span></div>
      <div className="workspace-label"><span className="workspace-avatar">P</span><div>Personal workspace<small>Stored on this device</small></div></div>
      <div className="nav-label">Workspace</div>
      <nav aria-label="Main navigation">
        <button className={view === 'ask' ? 'nav-button active' : 'nav-button'} onClick={() => setView('ask')}><Search size={18} />Ask your library</button>
        <button className={view === 'library' ? 'nav-button active' : 'nav-button'} onClick={() => setView('library')}><FolderOpen size={18} />Documents<span className="count">{documents.length}</span></button>
        <button className={view === 'runtime' ? 'nav-button active' : 'nav-button'} onClick={() => setView('runtime')}><Cpu size={18} />Local runtime</button>
      </nav>
      <div className="sidebar-bottom"><div className="privacy-card"><ShieldCheck size={20} /><strong>Private by design</strong><p>Your library and search index stay on this computer.</p></div><button className="runtime-mini" onClick={() => setView('runtime')}><span className={`status-dot ${engine?.connected ? 'online' : ''}`} /><span>{engine?.connected ? 'Local model connected' : 'Set up local inference'}</span><ChevronRight size={14} /></button><div className="version">Desktop preview <span>v0.1</span></div></div>
    </aside>

    <main className="main">
      <header className="topbar"><div><span>Workspace</span><ChevronRight size={13} /><strong>{view === 'ask' ? 'Ask your library' : view === 'library' ? 'Documents' : 'Local runtime'}</strong></div><span className="local-label"><HardDrive size={14} />Local storage</span></header>
      {!native && <div className="preview-banner"><CircleHelp size={17} /><span>Browser preview. Open the Tauri desktop app to import files and run local inference.</span></div>}
      {error && <div className="alert error" role="alert"><span>{error}</span><button aria-label="Dismiss error" onClick={() => setError('')}><X size={16} /></button></div>}
      {notice && <div className="alert" role="status"><Check size={17} /><span>{notice}</span><button aria-label="Dismiss notification" onClick={() => setNotice('')}><X size={16} /></button></div>}

      <div className="content-grid">
        <div className="content">
          {view === 'ask' && <>
            <div className={`ask-heading ${searched ? 'compact' : ''}`}><div className="section-eyebrow"><BookOpen size={16} />Knowledge, within reach</div><h1>What would you like<br />to find out?</h1><p>Bring your documents together. Follow every answer back to its source.</p></div>
            <form className="search-box" onSubmit={e => { e.preventDefault(); void search(); }}><label htmlFor="query" className="sr-only">Search your library</label><textarea id="query" placeholder="Ask a question about your documents…" rows={2} maxLength={2000} value={query} onChange={e => setQuery(e.target.value)} onKeyDown={e => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); void search(); } }} /><div className="search-toolbar"><span><Search size={14} />{semantic ? 'Semantic search' : 'Keyword search'} · {documents.length} documents</span><button className="send-button" aria-label="Search library" disabled={disabled || !query.trim()} type="submit">{busy === 'Searching' ? <LoaderCircle className="spin" size={18} /> : <ArrowUp size={19} />}</button></div></form>
            <label className="format-note"><input type="checkbox" checked={semantic} disabled={disabled} onChange={e => { setSemantic(e.target.checked); setHits([]); setAnswer(null); setSearched(''); }} /> Use semantic search (requires an indexed library and {memoryPolicy === 'single' ? 'an embedding model armed for on-demand startup' : 'a running embedding model'})</label>
            {!searched && <>
              <div className="suggestions">{starterQuestions.map(q => <button key={q} disabled={disabled || !documents.length} onClick={() => void search(q)}>{q}<ChevronRight size={15} /></button>)}</div>
              <div className="library-intro"><div><span className="illustration"><FileText size={29} strokeWidth={1.3} /><Search size={22} strokeWidth={1.4} /></span><h2>{documents.length ? 'Your library is ready to explore' : 'Start with something you know'}</h2><p>{documents.length ? `${documents.length} documents and ${totalChunks} indexed passages, available on this device.` : 'Import a document or explore a small sample workspace. Your files stay here.'}</p></div><div className="button-row"><button className="primary" disabled={disabled} onClick={() => void importFiles()}><Plus size={16} />Import documents</button><button className="secondary" disabled={disabled} onClick={() => void addSamples()}>Explore sample workspace</button></div><p className="format-note">Import text, Markdown, PDF, images, audio or video. Set up the media decoder and embedding model in Local runtime for media search.</p></div>
            </>}
            {searched && <div className="results">
              <div className="results-header"><h2>{hits.length ? `${hits.length} matching passages` : 'No matching passages'}</h2>{hits.length > 0 && <button className="primary" disabled={disabled} onClick={() => void generate()}>{busy === 'Generating locally' ? <LoaderCircle className="spin" size={16} /> : <Cpu size={16} />}Generate answer</button>}</div>
              {!hits.length && <p className="subtle">Try a more specific query or import relevant documents. Semantic results rank similarity; inspect the sources for supporting evidence.</p>}
              {busy === 'Generating locally' && <div className="answer-panel" role="status"><LoaderCircle size={18} className="spin" />The local model is reading the retrieved evidence. This request has a 120-second limit.</div>}
              {answer && <div className="answer-panel"><div className="answer-title"><Cpu size={17} /><strong>{answer.answer.insufficient_evidence ? 'More evidence needed' : 'Answer from your library'}</strong></div><p>{answer.answer.answer}</p><div className="answer-sources">{answer.sources.filter(h => answer.answer.source_ids.includes(h.id)).map((h, i) => <button key={h.id} onClick={() => void openSource(h.document_id, h.title, h.kind, h.page_number, h.start_ms, h.end_ms)} disabled={disabled}><FileText size={13} />{i + 1}. {h.title}{isPdf(h.kind) && h.page_number != null ? ` · Page ${h.page_number}` : ''}</button>)}</div><div className="metrics"><span>{(answer.elapsed_ms / 1000).toFixed(1)}s elapsed</span><span>{answer.prompt_tokens ?? '—'} input tokens</span><span>{answer.cached_tokens === null ? 'Cache tokens not reported' : `${answer.cached_tokens} cached tokens`}</span></div><small>Citations are checked against retrieved sources. Review the evidence for factual accuracy.</small></div>}
              <div className="result-list">{hits.map((hit, i) => <button className="result" key={hit.id} disabled={disabled} onClick={() => void openSource(hit.document_id, hit.title, hit.kind, hit.page_number, hit.start_ms, hit.end_ms)}><div className="result-top"><span><FileText size={16} />{hit.title}</span><span>{isMedia(hit.kind) ? `${hit.kind}${hit.start_ms != null ? ` · ${mediaTime(hit.start_ms)}` : ''}` : isPdf(hit.kind) ? `Page ${hit.page_number ?? 1}` : `Passage ${hit.ordinal + 1}`}<ChevronRight size={14} /></span></div><p>{hit.text.slice(0, 340)}{hit.text.length > 340 ? '…' : ''}</p><span className="result-number">Source {i + 1}</span></button>)}</div>
            </div>}
          </>}

          {view === 'library' && <>
            <div className="page-heading"><div><span className="section-eyebrow">Your local collection</span><h1>Documents</h1><p>Text, PDF and media previews with searchable segments, stored together.</p></div><button className="primary" disabled={disabled} onClick={() => void importFiles()}><Import size={17} />Import files</button></div>
            <div className="library-stats"><span><strong>{documents.length}</strong> documents</span><span><strong>{totalChunks}</strong> passages</span><span><strong>{displayBytes(documents.reduce((s, d) => s + d.bytes, 0))}</strong> stored source data</span></div>
            {documents.length === 0 ? <div className="empty-library"><FolderOpen size={36} strokeWidth={1.2} /><h2>A home for your knowledge</h2><p>Import text, PDF or supported media. Configure FFmpeg in Local runtime for media.</p><button className="secondary" disabled={disabled} onClick={() => void addSamples()}>Add synthetic sample documents</button></div> : <div className="document-list">{documents.map(doc => <div className="document-row" key={doc.id}><button className="document-open" onClick={() => void openSource(doc.id, doc.title, doc.kind)} disabled={disabled}><span className="file-icon"><FileText size={21} /></span><span><strong>{doc.title}</strong><small>{doc.chunks} passages · {displayBytes(doc.bytes)} · {new Date(doc.created_at).toLocaleDateString()}</small></span></button>{deleteId === doc.id ? <div className="delete-confirm"><span>Remove from library?</span><button disabled={disabled} onClick={() => void remove(doc.id)}>Remove</button><button onClick={() => setDeleteId(null)}>Cancel</button></div> : <button className="icon-button" aria-label={`Remove ${doc.title}`} disabled={disabled} onClick={() => setDeleteId(doc.id)}><Trash2 size={16} /></button>}</div>)}</div>}
            <div className="info-note"><ShieldCheck size={18} /><p>Removing a document deletes its local index, stored text and media previews. The original file you imported remains on disk.</p></div>
          </>}

          {view === 'runtime' && <>
            <div className="page-heading"><div><span className="section-eyebrow">On-device intelligence</span><h1>Local runtime</h1><p>Connect a quantized model running on this computer.</p></div><Cpu size={36} strokeWidth={1.2} /></div>
            {native && <MemoryPolicyControl disabled={disabled} onChange={setMemoryPolicy} />}
            {native && <ManagedRuntime disabled={disabled} singleResident={memoryPolicy === 'single'} onConnectionChange={running => { const revision = ++engineRevision.current; setEngine(null); if (running) { invoke<Engine>('engine_status').then(status => { if (engineRevision.current === revision) setEngine(status); }).catch(e => { if (engineRevision.current === revision) setError(String(e)); }); } }} />}
            <section className="settings-card"><div className="settings-title"><Cpu size={19} /><h2>llama.cpp connection</h2><span className="status-text">{engine?.connected ? 'Connected' : 'Awaiting connection'}</span></div><p>Start your local llama-server with a supported GGUF model. Cachella connects only to direct loopback addresses.</p><label htmlFor="endpoint">Local server address</label><div className="endpoint-row"><input id="endpoint" value={endpoint} onChange={e => { setEndpoint(e.target.value); engineRevision.current += 1; setEngine(null); }} spellCheck={false} /><button className="primary" disabled={disabled} onClick={() => void checkEngine()}>{busy === 'Checking local model' ? <LoaderCircle className="spin" size={15} /> : <Check size={16} />}Save & check</button></div>{engine && <div className="connection-result" role="status"><span className={`status-dot ${engine.connected ? 'online' : ''}`} /><span>{engine.model ?? engine.message}</span></div>}<div className="command-example"><span>Example for an already downloaded model</span><code>llama-server -m /path/to/model.gguf --host 127.0.0.1 --port 8080 -c 4096 -np 1</code></div><p className="format-note">For an independently managed server, stop the Cachella model and use Save & check.</p></section>
            {native && <><MediaSetup disabled={disabled} /><EmbeddingSetup disabled={disabled} singleResident={memoryPolicy === 'single'} /></>}
            <section className="settings-card"><div className="settings-title"><Layers size={19} /><h2>Prompt reuse</h2></div><div className="toggle-row"><div><strong>Allow the engine to reuse matching prefixes</strong><p>Applied to the next answer request. Returned cache counters appear below the answer when available.</p></div><label className="switch"><input type="checkbox" checked={cache} onChange={e => setCache(e.target.checked)} aria-label="Enable prompt cache reuse" /><span /></label></div><div className="info-note"><CircleHelp size={17} /><p>This is llama.cpp caching. LMCache is an optional future enterprise-server integration.</p></div></section>
            <section className="settings-card"><div className="settings-title"><HardDrive size={19} /><h2>Storage and capabilities</h2></div><dl className="settings-facts"><div><dt>Library location</dt><dd>{workspace.data_path || 'Available in the desktop application'}</dd></div><div><dt>Current search</dt><dd>SQLite full-text search and optional local semantic search</dd></div><div><dt>Current imports</dt><dd>Text, Markdown, PDF, PNG/JPEG, audio and video</dd></div><div><dt>Answers</dt><dd>Text and retrieved media with compatible local models</dd></div></dl></section>
          </>}
        </div>
        {source && <aside className="source-panel" aria-label="Source document"><div className="source-panel-header"><span><FileText size={16} />Source preview</span><button className="icon-button" aria-label="Close source preview" onClick={() => setSource(null)}><X size={18} /></button></div><h2>{source.title}</h2>{isPdf(source.kind) ? <PdfPreview key={`${source.id}:${source.pageNumber ?? 1}`} id={source.id} initialPage={source.pageNumber} /> : <><div className="source-body">{source.text}</div>{isMedia(source.kind) && <MediaPreview key={`${source.id}:${source.startMs ?? 0}`} id={source.id} title={source.title} kind={source.kind!} startMs={source.startMs} endMs={source.endMs} />}</>}<div className="source-footer"><ShieldCheck size={14} />{isMedia(source.kind) ? 'Media preview stored locally' : isPdf(source.kind) ? 'Extracted PDF text stored locally' : 'Stored in your local library'}</div></aside>}
      </div>
      <footer className="app-footer"><span>{busy ? <><LoaderCircle className="spin" size={13} />{busy}</> : <><span className="status-dot online" />{native ? 'Workspace ready' : 'Interface preview'}</>}</span>{answerOperation ? <button className="cancel-answer" disabled={answerOperation.cancelling} onClick={() => { const operation = activeAnswer.current; if (operation) void cancelAnswer(operation).catch(() => undefined); }}><X size={13} />{answerOperation.cancelling ? 'Cancelling answer' : 'Cancel answer'}</button> : searchOperation ? <button className="cancel-answer" disabled={searchOperation.cancelling} onClick={() => { const operation = activeSearch.current; if (operation) void cancelSearch(operation).catch(() => undefined); }}><X size={13} />{searchOperation.cancelling ? 'Cancelling search' : 'Cancel search'}</button> : <button onClick={() => setView('runtime')}><Settings2 size={13} />Runtime & privacy</button>}</footer>
    </main>
  </div>;
}
