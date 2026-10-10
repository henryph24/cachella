import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

type PdfPage = { number: number; text: string };

export default function PdfPreview({ id, initialPage = 1 }: { id: string; initialPage?: number | null }) {
  const [pages, setPages] = useState<PdfPage[]>([]);
  const [pageNumber, setPageNumber] = useState(initialPage ?? 1);
  const [error, setError] = useState('');
  useEffect(() => {
    let active = true;
    setPages([]); setError(''); setPageNumber(initialPage ?? 1);
    invoke<PdfPage[]>('read_pdf_pages', { id }).then(result => { if (active) setPages(result); }).catch(reason => { if (active) setError(String(reason)); });
    return () => { active = false; };
  }, [id, initialPage]);
  const selectedIndex = Math.max(0, pages.findIndex(page => page.number === pageNumber));
  const page = pages[selectedIndex];
  if (error) return <p className="format-note" role="alert">PDF pages could not be opened: {error}</p>;
  if (!pages.length) return <p className="format-note">Loading extracted PDF pages…</p>;
  return <div className="pdf-preview">
    <div className="pdf-page-controls" aria-label="PDF page navigation">
      <button aria-label="Previous page" disabled={selectedIndex === 0} onClick={() => setPageNumber(pages[selectedIndex - 1].number)}>Previous</button>
      <label>Page <select aria-label="PDF page" value={page.number} onChange={event => setPageNumber(Number(event.target.value))}>{pages.map(candidate => <option key={candidate.number} value={candidate.number}>{candidate.number}</option>)}</select> <span>of {pages.length}</span></label>
      <button aria-label="Next page" disabled={selectedIndex === pages.length - 1} onClick={() => setPageNumber(pages[selectedIndex + 1].number)}>Next</button>
    </div>
    <h3>Page {page.number}</h3>
    <div className="source-body pdf-page-text">{page.text || 'No extracted text for this page.'}</div>
    <p className="format-note">Extracted text can vary from the original layout. Scanned pages require OCR, which is not available.</p>
  </div>;
}
