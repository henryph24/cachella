# Data source experience

Requirement recorded 10 October 2026: users should be able to bring existing
knowledge into Cachella from Obsidian, Notion, Google Drive and other relevant
sources with minimal setup. This document describes proposed behavior. Cloud
connectors and watched-vault import are not implemented or qualified yet.

## User flow

1. Choose Add source: files, folder/Obsidian vault, Notion, or Google Drive.
2. For local sources, use the native folder picker. For cloud sources, connect an
   account in the system browser and return to Cachella. Show the connected identity.
3. Select folders or pages and preview supported items, exclusions and estimated
   download size. Default to read-only access and explicitly selected content.
4. Import with item counts, cancellable progress and per-item errors. Start making
   completed items searchable while the remaining items are processed.
5. Show last successful sync, pending changes, retry and disconnect controls.
   Search results retain a link to the original source and its modification time.

## Proposed responsibilities

| Component | Responsibility |
| --- | --- |
| Desktop folder adapter | Read selected folders and Obsidian Markdown/attachments; preserve relative paths and links; rescan on restart and reconcile watcher events |
| Composio cloud adapter | OAuth connection lifecycle and supported provider read operations for Notion and Drive |
| Small authenticated backend | Hold the project API key, bind authenticated users to their own connections, allowlist read operations and route change notifications |
| Desktop ingestion pipeline | Normalize content, enforce existing media limits, chunk, embed, store and search locally |
| Source reconciliation | Track stable provider IDs, versions, hashes, pagination checkpoints, deletions and permission changes |

Composio provides useful connector infrastructure. A connector catalog alone does
not establish complete recursive export, attachment coverage or continuous sync.
Verify provider-specific content and event coverage with real test accounts before
advertising a source as supported. Use deterministic SDK/API calls for importing.
An LLM does not need to choose which ingestion operations run.

## Sync and authorization acceptance

- Repeating an import creates no duplicate source records or embeddings.
- Edits replace affected chunks; renames preserve source identity where the
  provider exposes a stable ID. Unchanged content reuses its embeddings.
- Interrupted imports resume from committed checkpoints. Pagination, rate limits
  and partial failures preserve a visible, truthful sync status.
- Source deletion, account revocation and confirmed access loss remove affected
  content from retrieval and invalidate associated cached answers.
- Define an offline access policy explicitly. Cached documents cannot learn of
  remote permission revocation while the device is disconnected; enterprise
  policies may require access expiry and revalidation before retrieval.
- Disconnect explains whether it removes local copies; provide an explicit local
  purge that removes chunks, vectors, previews and related cached answers.
- Test separate users and tenants, OAuth callback identity, webhook authenticity,
  replayed events and reconnection. Keep project secrets on the backend.
- Cover nested Notion blocks and databases, Drive native-document export and
  selected shared-drive access, Obsidian Unicode paths, links and attachments.
- Reject traversal and out-of-scope symlinks; ignore vault configuration/secrets by
  default. Preserve the existing per-file and media decoding limits.

## Budget and data processing

For a demo, evaluate Composio Hobby within its hard usage caps. The current price
page lists 100,000 calls/month, with 20,000 of those available through managed
OAuth apps. Direct execution has a separate 10,000-call free allowance. Pro starts
at $29/month, exceeding the current total monthly budget before hosting.
Track actual operations per document: listing, paginated blocks and attachments
can each require additional calls. Backend hosting must fit the remaining budget.

Cloud imports are processed through Composio and our backend. Its default logs
store tool arguments/results. Local embeddings and inference do not eliminate
that cloud processing. Explain the data path before connection; retain an entirely
local folder-import option. Paid zero-data-retention coverage has additional scope
limits and requires separate evaluation for enterprise data.

## Sources checked

- [Google Drive toolkit](https://docs.composio.dev/toolkits/googledrive)
- [Notion toolkit and access guidance](https://docs.composio.dev/kb/guide/toolkits-notion)
- [Drive integration guidance](https://docs.composio.dev/kb/guide/toolkits-googledrive)
- [Composio pricing](https://composio.dev/pricing)
- [Token custody and application authorization](https://docs.composio.dev/docs/security/token-custody)
- [Zero-data-retention scope](https://docs.composio.dev/docs/security/zero-data-retention)

No Composio account was connected, no content was transmitted and no paid plan was
enabled for this research. Live connector qualification requires test accounts.
