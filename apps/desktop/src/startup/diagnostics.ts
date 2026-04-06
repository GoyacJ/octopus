export type StartupFailureSnapshot = {
  title: string
  message: string
  stack: string
}

const STARTUP_FAILURE_TITLE = 'Desktop startup failed'

export function describeStartupFailure(error: unknown): StartupFailureSnapshot {
  if (error instanceof Error) {
    return {
      title: STARTUP_FAILURE_TITLE,
      message: error.message || 'Unknown startup error',
      stack: error.stack ?? '',
    }
  }

  return {
    title: STARTUP_FAILURE_TITLE,
    message: typeof error === 'string' ? error : 'Unknown startup error',
    stack: '',
  }
}

function ensureStartupRoot(): HTMLElement | null {
  return document.querySelector<HTMLElement>('#app')
}

function escapeHtml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;')
}

export function renderStartupFailure(error: unknown): void {
  const root = ensureStartupRoot()
  if (!root) {
    return
  }

  const snapshot = describeStartupFailure(error)
  const stack = snapshot.stack
    ? `<pre style="margin:12px 0 0;max-height:40vh;overflow:auto;padding:12px;border-radius:12px;background:#f5f5f4;color:#292524;font-size:12px;line-height:1.5;white-space:pre-wrap;">${escapeHtml(snapshot.stack)}</pre>`
    : ''

  root.innerHTML = `
    <div style="min-height:100vh;display:flex;align-items:center;justify-content:center;background:#f8f7f4;padding:24px;font-family:Inter,system-ui,-apple-system,sans-serif;color:#1c1917;">
      <section style="width:min(760px,100%);border:1px solid rgba(28,25,23,0.12);border-radius:24px;background:#ffffff;padding:28px;box-shadow:0 24px 64px rgba(15,23,42,0.08);">
        <p style="margin:0 0 8px;font-size:11px;font-weight:700;letter-spacing:0.24em;text-transform:uppercase;color:#78716c;">Fatal startup error</p>
        <h1 style="margin:0 0 12px;font-size:28px;line-height:1.2;">${escapeHtml(snapshot.title)}</h1>
        <p style="margin:0;font-size:14px;line-height:1.7;color:#57534e;">${escapeHtml(snapshot.message)}</p>
        ${stack}
      </section>
    </div>
  `
}

export function installStartupDiagnostics(): void {
  window.addEventListener('error', (event) => {
    renderStartupFailure(event.error ?? event.message)
  })

  window.addEventListener('unhandledrejection', (event) => {
    renderStartupFailure(event.reason)
  })
}
